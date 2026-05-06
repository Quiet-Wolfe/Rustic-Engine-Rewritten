//! CPAL output stream wiring around the headless mixer.

use crate::error::{AudioError, AudioResult};
use crate::{MixStats, Mixer, SoundSource, Stem, VoiceId};
use cpal::traits::{DeviceTrait, HostTrait, StreamTrait};
use cpal::{FromSample, Sample, SampleFormat, SizedSample, I24, U24};
use rustic_core::time::Samples;
use std::sync::{Arc, Mutex};

#[derive(Debug, Clone)]
pub struct SharedMixer {
    inner: Arc<Mutex<Mixer>>,
}

impl SharedMixer {
    pub fn new(mixer: Mixer) -> Self {
        Self {
            inner: Arc::new(Mutex::new(mixer)),
        }
    }

    #[inline]
    pub fn sample_cursor(&self) -> Samples {
        self.inner
            .lock()
            .map(|mixer| mixer.sample_cursor())
            .unwrap_or(Samples(0))
    }

    #[inline]
    pub fn sample_rate(&self) -> u32 {
        self.inner
            .lock()
            .map(|mixer| mixer.sample_rate())
            .unwrap_or(1)
    }

    pub fn edit<R>(&self, f: impl FnOnce(&mut Mixer) -> AudioResult<R>) -> AudioResult<R> {
        let mut mixer = self
            .inner
            .lock()
            .map_err(|_| AudioError::Mix("mixer lock poisoned".to_string()))?;
        f(&mut mixer)
    }

    pub fn add_source(&self, stem: Stem, source: SoundSource) -> AudioResult<VoiceId> {
        self.edit(|mixer| mixer.add_source(stem, source))
    }

    pub fn seek(&self, position: Samples) -> AudioResult<()> {
        self.edit(|mixer| mixer.seek(position))
    }

    pub fn mix_stereo(&self, out: &mut [f32]) -> AudioResult<MixStats> {
        self.edit(|mixer| mixer.mix_stereo(out))
    }
}

pub struct AudioOutput {
    mixer: SharedMixer,
    stream: cpal::Stream,
    sample_rate: u32,
    channels: u16,
    sample_format: SampleFormat,
}

impl AudioOutput {
    pub fn open_default() -> AudioResult<Self> {
        let host = cpal::default_host();
        let device = host
            .default_output_device()
            .ok_or_else(|| AudioError::DeviceUnavailable("no default output device".to_string()))?;
        let supported = device
            .default_output_config()
            .map_err(|err| AudioError::DeviceUnavailable(format!("default config: {err}")))?;
        let sample_format = supported.sample_format();
        let config = supported.config();
        let channels = config.channels;
        if channels == 0 {
            return Err(AudioError::DeviceUnavailable(
                "default output config has zero channels".to_string(),
            ));
        }

        let mixer = SharedMixer::new(Mixer::new(config.sample_rate));
        let stream = build_output_stream(&device, &config, sample_format, mixer.clone())?;
        stream
            .play()
            .map_err(|err| AudioError::Stream(format!("play output stream: {err}")))?;

        Ok(Self {
            mixer,
            stream,
            sample_rate: config.sample_rate,
            channels,
            sample_format,
        })
    }

    #[inline]
    pub fn mixer(&self) -> &SharedMixer {
        &self.mixer
    }

    #[inline]
    pub fn sample_rate(&self) -> u32 {
        self.sample_rate
    }

    #[inline]
    pub fn channels(&self) -> u16 {
        self.channels
    }

    #[inline]
    pub fn sample_format(&self) -> SampleFormat {
        self.sample_format
    }

    #[inline]
    pub fn stream(&self) -> &cpal::Stream {
        &self.stream
    }
}

fn build_output_stream(
    device: &cpal::Device,
    config: &cpal::StreamConfig,
    sample_format: SampleFormat,
    mixer: SharedMixer,
) -> AudioResult<cpal::Stream> {
    match sample_format {
        SampleFormat::I8 => build_typed_output_stream::<i8>(device, config, mixer),
        SampleFormat::I16 => build_typed_output_stream::<i16>(device, config, mixer),
        SampleFormat::I24 => build_typed_output_stream::<I24>(device, config, mixer),
        SampleFormat::I32 => build_typed_output_stream::<i32>(device, config, mixer),
        SampleFormat::I64 => build_typed_output_stream::<i64>(device, config, mixer),
        SampleFormat::U8 => build_typed_output_stream::<u8>(device, config, mixer),
        SampleFormat::U16 => build_typed_output_stream::<u16>(device, config, mixer),
        SampleFormat::U24 => build_typed_output_stream::<U24>(device, config, mixer),
        SampleFormat::U32 => build_typed_output_stream::<u32>(device, config, mixer),
        SampleFormat::U64 => build_typed_output_stream::<u64>(device, config, mixer),
        SampleFormat::F32 => build_typed_output_stream::<f32>(device, config, mixer),
        SampleFormat::F64 => build_typed_output_stream::<f64>(device, config, mixer),
        other => Err(AudioError::DeviceUnavailable(format!(
            "unsupported output sample format {other}"
        ))),
    }
}

fn build_typed_output_stream<T>(
    device: &cpal::Device,
    config: &cpal::StreamConfig,
    mixer: SharedMixer,
) -> AudioResult<cpal::Stream>
where
    T: SizedSample + FromSample<f32>,
{
    let channels = usize::from(config.channels);
    let mut scratch = Vec::new();
    device
        .build_output_stream(
            config,
            move |output: &mut [T], _| write_output(output, channels, &mixer, &mut scratch),
            move |_| {},
            None,
        )
        .map_err(|err| AudioError::Stream(format!("build output stream: {err}")))
}

fn write_output<T>(output: &mut [T], channels: usize, mixer: &SharedMixer, scratch: &mut Vec<f32>)
where
    T: Sample + FromSample<f32>,
{
    if channels == 0 {
        return;
    }

    let frames = output.len() / channels;
    scratch.resize(frames * 2, 0.0);
    if mixer.mix_stereo(scratch).is_err() {
        scratch.fill(0.0);
    }

    let mut chunks = output.chunks_exact_mut(channels);
    for (frame_index, frame) in chunks.by_ref().enumerate() {
        let stereo = frame_index * 2;
        let left = scratch[stereo].clamp(-1.0, 1.0);
        let right = scratch[stereo + 1].clamp(-1.0, 1.0);
        for (channel, sample) in frame.iter_mut().enumerate() {
            let value = match channel {
                0 => left,
                1 => right,
                _ => (left + right) * 0.5,
            };
            *sample = T::from_sample(value);
        }
    }
    for sample in chunks.into_remainder() {
        *sample = T::from_sample(0.0);
    }
}

#[cfg(test)]
#[allow(clippy::unwrap_used, clippy::expect_used)]
mod tests {
    use super::*;
    use crate::SoundSource;

    #[test]
    fn callback_mixes_shared_mixer_into_device_buffer() {
        let mixer = SharedMixer::new(Mixer::new(48_000));
        mixer
            .add_source(
                Stem::Instrumental,
                SoundSource::Pcm(Arc::from([0.5, -0.5, 0.25, -0.25])),
            )
            .unwrap();
        let mut out = [0.0_f32; 4];
        let mut scratch = Vec::new();

        write_output(&mut out, 2, &mixer, &mut scratch);

        assert_eq!(out, [0.5, -0.5, 0.25, -0.25]);
        assert_eq!(mixer.sample_cursor(), Samples(2));
    }

    #[test]
    fn callback_fills_extra_channels_from_stereo_average() {
        let mixer = SharedMixer::new(Mixer::new(48_000));
        mixer
            .add_source(
                Stem::Instrumental,
                SoundSource::Pcm(Arc::from([0.5, -0.25])),
            )
            .unwrap();
        let mut out = [0.0_f32; 4];
        let mut scratch = Vec::new();

        write_output(&mut out, 4, &mixer, &mut scratch);

        assert_eq!(out, [0.5, -0.25, 0.125, 0.125]);
        assert_eq!(mixer.sample_cursor(), Samples(1));
    }
}
