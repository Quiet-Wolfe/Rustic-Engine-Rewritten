//! Headless custom mixer. The `cpal` device callback will call into this
//! mixer; gameplay reads its integer sample cursor as the authoritative
//! song clock.
// LINT-ALLOW: long-file mixer implementation plus focused unit tests

use crate::error::{AudioError, AudioResult};
use crate::source::{Decoder, SoundSource};
use rustic_core::time::Samples;

const OUTPUT_CHANNELS: usize = 2;
const DEFAULT_PCM_CHANNELS: u16 = 2;

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub struct VoiceId(u64);

impl VoiceId {
    pub const fn new(id: u64) -> Self {
        Self(id)
    }

    pub const fn get(self) -> u64 {
        self.0
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
#[non_exhaustive]
pub enum Stem {
    Instrumental,
    Vocals,
    Sfx,
    Custom(u16),
}

#[derive(Debug, Clone, Copy, PartialEq)]
#[non_exhaustive]
pub struct MixStats {
    pub frames: usize,
    pub sample_cursor: Samples,
    pub active_voices: usize,
}

#[derive(Debug)]
pub struct Mixer {
    sample_cursor: Samples,
    sample_rate: u32,
    paused: bool,
    next_voice_id: u64,
    voices: Vec<Voice>,
    stem_gains: Vec<(Stem, f32)>,
}

impl Default for Mixer {
    fn default() -> Self {
        Self::new(48_000)
    }
}

impl Mixer {
    pub fn new(sample_rate: u32) -> Self {
        Self {
            sample_cursor: Samples(0),
            sample_rate: sample_rate.max(1),
            paused: false,
            next_voice_id: 1,
            voices: Vec::new(),
            stem_gains: Vec::new(),
        }
    }

    #[inline]
    pub fn sample_cursor(&self) -> Samples {
        self.sample_cursor
    }

    #[inline]
    pub fn sample_rate(&self) -> u32 {
        self.sample_rate
    }

    #[inline]
    pub fn is_paused(&self) -> bool {
        self.paused
    }

    #[inline]
    pub fn voice_count(&self) -> usize {
        self.voices.len()
    }

    pub fn pause(&mut self) {
        self.paused = true;
    }

    pub fn resume(&mut self) {
        self.paused = false;
    }

    pub fn clear(&mut self) {
        self.voices.clear();
    }

    /// Add a source at its own start. Song stems are normally added while
    /// the mixer cursor is at zero, then moved with `seek` for restarts or
    /// dev rewind.
    pub fn add_source(&mut self, stem: Stem, source: SoundSource) -> AudioResult<VoiceId> {
        self.add_source_with_loop(stem, source, false)
    }

    pub fn add_looped_source(&mut self, stem: Stem, source: SoundSource) -> AudioResult<VoiceId> {
        self.add_source_with_loop(stem, source, true)
    }

    fn add_source_with_loop(
        &mut self,
        stem: Stem,
        source: SoundSource,
        looped: bool,
    ) -> AudioResult<VoiceId> {
        let id = VoiceId::new(self.next_voice_id);
        self.next_voice_id += 1;
        self.voices.push(Voice::from_source(
            id,
            stem,
            source,
            self.sample_rate,
            looped,
        )?);
        Ok(id)
    }

    pub fn stop_voice(&mut self, id: VoiceId) -> bool {
        let before = self.voices.len();
        self.voices.retain(|v| v.id != id);
        self.voices.len() != before
    }

    pub fn has_voice(&self, id: VoiceId) -> bool {
        self.voices.iter().any(|v| v.id == id)
    }

    pub fn set_voice_gain(&mut self, id: VoiceId, gain: f32) -> bool {
        let Some(voice) = self.voices.iter_mut().find(|v| v.id == id) else {
            return false;
        };
        voice.gain = gain.max(0.0);
        true
    }

    pub fn set_stem_gain(&mut self, stem: Stem, gain: f32) {
        let gain = gain.max(0.0);
        if let Some((_, existing)) = self.stem_gains.iter_mut().find(|(s, _)| *s == stem) {
            *existing = gain;
        } else {
            self.stem_gains.push((stem, gain));
        }
    }

    pub fn stem_gain(&self, stem: Stem) -> f32 {
        stem_gain(&self.stem_gains, stem)
    }

    /// Render interleaved stereo frames and advance the authoritative
    /// sample cursor by the number of frames mixed. Paused playback emits
    /// silence without advancing source positions or the cursor.
    pub fn mix_stereo(&mut self, out: &mut [f32]) -> AudioResult<MixStats> {
        if !out.len().is_multiple_of(OUTPUT_CHANNELS) {
            return Err(AudioError::Mix(
                "stereo output buffer must contain whole frames".to_string(),
            ));
        }

        out.fill(0.0);
        let frames = out.len() / OUTPUT_CHANNELS;
        if frames == 0 {
            return Ok(MixStats {
                frames,
                sample_cursor: self.sample_cursor,
                active_voices: self.voices.len(),
            });
        }

        let stem_gains = self.stem_gains.clone();
        for voice in &mut self.voices {
            if self.paused && voice.stem != Stem::Sfx {
                continue;
            }
            let gain = stem_gain(&stem_gains, voice.stem);
            voice.mix_into(self.sample_rate, out, gain)?;
        }
        self.voices.retain(|v| !v.ended);
        if !self.paused {
            self.sample_cursor = Samples(self.sample_cursor.0 + frames as i64);
        }

        Ok(MixStats {
            frames,
            sample_cursor: self.sample_cursor,
            active_voices: self.voices.len(),
        })
    }

    /// Seeking sets the mixer cursor directly and seeks every active source
    /// to the equivalent source-domain frame.
    pub fn seek(&mut self, position: Samples) -> AudioResult<()> {
        let position = Samples(position.0.max(0));
        self.sample_cursor = position;
        for voice in &mut self.voices {
            voice.seek_mixer_position(position, self.sample_rate)?;
        }
        Ok(())
    }

    /// Seek the song cursor and only sources on the requested stems. SFX voices
    /// keep their source position, which lets menu/game-over music continue
    /// while gameplay stems are reset under a paused mixer.
    pub fn seek_stems(&mut self, position: Samples, stems: &[Stem]) -> AudioResult<()> {
        let position = Samples(position.0.max(0));
        self.sample_cursor = position;
        for voice in &mut self.voices {
            if stems.contains(&voice.stem) {
                voice.seek_mixer_position(position, self.sample_rate)?;
            }
        }
        Ok(())
    }

    /// Advance the authoritative sample cursor without touching active
    /// sources. This remains available for gameplay/input unit tests that
    /// need a cheap fake clock.
    pub fn advance_for_test(&mut self, frames: i64) {
        self.sample_cursor = Samples(self.sample_cursor.0 + frames);
    }

    pub fn seek_for_test(&mut self, position: Samples) {
        self.sample_cursor = position;
    }
}

fn stem_gain(gains: &[(Stem, f32)], stem: Stem) -> f32 {
    gains
        .iter()
        .find_map(|(s, gain)| (*s == stem).then_some(*gain))
        .unwrap_or(1.0)
}

#[derive(Debug)]
struct Voice {
    id: VoiceId,
    stem: Stem,
    source: SoundSource,
    source_rate: u32,
    channels: u16,
    source_pos: f64,
    gain: f32,
    looped: bool,
    ended: bool,
    stream_buffer: Vec<f32>,
    stream_buffer_start: i64,
    stream_ended: bool,
}

impl Voice {
    fn from_source(
        id: VoiceId,
        stem: Stem,
        source: SoundSource,
        mixer_rate: u32,
        looped: bool,
    ) -> AudioResult<Self> {
        let (source_rate, channels) = match &source {
            SoundSource::Pcm(samples) => {
                if samples.len() % OUTPUT_CHANNELS != 0 {
                    return Err(AudioError::InvalidSource(
                        "PCM sources must be interleaved stereo".to_string(),
                    ));
                }
                (mixer_rate, DEFAULT_PCM_CHANNELS)
            }
            SoundSource::Streaming(decoder) => {
                let sample_rate = decoder.sample_rate();
                let channels = decoder.channels();
                if sample_rate == 0 {
                    return Err(AudioError::InvalidSource(
                        "streaming source has a zero sample rate".to_string(),
                    ));
                }
                if channels == 0 {
                    return Err(AudioError::InvalidSource(
                        "streaming source has zero channels".to_string(),
                    ));
                }
                (sample_rate, channels)
            }
        };

        Ok(Self {
            id,
            stem,
            source,
            source_rate,
            channels,
            source_pos: 0.0,
            gain: 1.0,
            looped,
            ended: false,
            stream_buffer: Vec::new(),
            stream_buffer_start: 0,
            stream_ended: false,
        })
    }

    fn mix_into(&mut self, mixer_rate: u32, out: &mut [f32], stem_gain: f32) -> AudioResult<()> {
        let frames = out.len() / OUTPUT_CHANNELS;
        let source_step = self.source_rate as f64 / mixer_rate as f64;
        let gain = self.gain * stem_gain;

        for frame in 0..frames {
            let mut base_frame = self.source_pos.floor() as i64;
            let pair = match self.frame_pair(base_frame)? {
                Some(pair) => Some(pair),
                None if self.looped => {
                    self.restart_loop()?;
                    base_frame = 0;
                    self.frame_pair(base_frame)?
                }
                None => None,
            };
            let Some((l0, r0)) = pair else {
                self.ended = true;
                break;
            };
            let frac = self.source_pos - base_frame as f64;
            let (l1, r1) = self.frame_pair(base_frame + 1)?.unwrap_or((l0, r0));
            let left = lerp(l0, l1, frac) * gain;
            let right = lerp(r0, r1, frac) * gain;
            let out_i = frame * OUTPUT_CHANNELS;
            out[out_i] += left;
            out[out_i + 1] += right;
            self.source_pos += source_step;
        }

        self.compact_stream_buffer();
        Ok(())
    }

    fn restart_loop(&mut self) -> AudioResult<()> {
        self.source_pos = 0.0;
        self.ended = false;
        self.stream_buffer.clear();
        self.stream_buffer_start = 0;
        self.stream_ended = false;
        if let SoundSource::Streaming(decoder) = &mut self.source {
            decoder.seek(Samples(0))?;
        }
        Ok(())
    }

    fn seek_mixer_position(&mut self, position: Samples, mixer_rate: u32) -> AudioResult<()> {
        let source_frame = ((position.0 as f64 * self.source_rate as f64) / mixer_rate as f64)
            .round()
            .max(0.0) as i64;
        self.source_pos = source_frame as f64;
        self.ended = false;
        self.stream_buffer.clear();
        self.stream_buffer_start = source_frame;
        self.stream_ended = false;

        if let SoundSource::Streaming(decoder) = &mut self.source {
            decoder.seek(Samples(source_frame))?;
        }
        Ok(())
    }

    fn frame_pair(&mut self, frame: i64) -> AudioResult<Option<(f32, f32)>> {
        if frame < 0 {
            return Ok(None);
        }

        match &mut self.source {
            SoundSource::Pcm(samples) => {
                let index = frame as usize * OUTPUT_CHANNELS;
                if index + 1 >= samples.len() {
                    return Ok(None);
                }
                Ok(Some((samples[index], samples[index + 1])))
            }
            SoundSource::Streaming(decoder) => {
                let channels = usize::from(self.channels);
                ensure_streaming_frame(
                    decoder.as_mut(),
                    channels,
                    &mut self.stream_buffer,
                    self.stream_buffer_start,
                    &mut self.stream_ended,
                    frame,
                )?;

                let end =
                    stream_buffer_end(self.stream_buffer_start, &self.stream_buffer, channels);
                if frame < self.stream_buffer_start || frame >= end {
                    return Ok(None);
                }

                let offset = (frame - self.stream_buffer_start) as usize * channels;
                let left = self.stream_buffer[offset];
                let right = if channels == 1 {
                    left
                } else {
                    self.stream_buffer[offset + 1]
                };
                Ok(Some((left, right)))
            }
        }
    }

    fn compact_stream_buffer(&mut self) {
        if !matches!(self.source, SoundSource::Streaming(_)) {
            return;
        }

        let keep_from = self.source_pos.floor().max(0.0) as i64;
        let drop_frames = (keep_from - self.stream_buffer_start).max(0);
        if drop_frames == 0 {
            return;
        }

        let channels = usize::from(self.channels);
        let drop_samples = drop_frames as usize * channels;
        let drop_samples = drop_samples.min(self.stream_buffer.len());
        self.stream_buffer.drain(0..drop_samples);
        self.stream_buffer_start += drop_frames;
    }
}

fn ensure_streaming_frame(
    decoder: &mut dyn Decoder,
    channels: usize,
    buffer: &mut Vec<f32>,
    buffer_start: i64,
    stream_ended: &mut bool,
    frame: i64,
) -> AudioResult<()> {
    while frame >= stream_buffer_end(buffer_start, buffer, channels) && !*stream_ended {
        let buffered_end = stream_buffer_end(buffer_start, buffer, channels);
        let requested_frames = (frame - buffered_end + 1).max(1) as usize;
        let mut tmp = vec![0.0; requested_frames * channels];
        let read_frames = decoder.read(&mut tmp)?;
        if read_frames == 0 {
            *stream_ended = true;
            break;
        }
        tmp.truncate(read_frames * channels);
        buffer.extend_from_slice(&tmp);
    }
    Ok(())
}

fn stream_buffer_end(buffer_start: i64, buffer: &[f32], channels: usize) -> i64 {
    buffer_start + (buffer.len() / channels) as i64
}

fn lerp(a: f32, b: f32, t: f64) -> f32 {
    a + (b - a) * t as f32
}

#[cfg(test)]
#[allow(clippy::unwrap_used, clippy::expect_used)]
mod tests {
    use super::*;
    use std::sync::{Arc, Mutex};

    #[test]
    fn mixes_stereo_pcm_and_advances_cursor() {
        let mut mixer = Mixer::new(48_000);
        let samples: Arc<[f32]> = Arc::from([1.0, 0.5, 0.25, 0.125]);
        mixer
            .add_source(Stem::Instrumental, SoundSource::Pcm(samples))
            .unwrap();
        let mut out = [0.0; 4];

        let stats = mixer.mix_stereo(&mut out).unwrap();

        assert_eq!(out, [1.0, 0.5, 0.25, 0.125]);
        assert_eq!(stats.frames, 2);
        assert_eq!(stats.sample_cursor, Samples(2));
        assert_eq!(mixer.sample_cursor(), Samples(2));
    }

    #[test]
    fn supports_independent_instrumental_and_vocal_stems() {
        let mut mixer = Mixer::new(48_000);
        let inst: Arc<[f32]> = Arc::from([0.5, 0.5]);
        let vocals: Arc<[f32]> = Arc::from([0.25, -0.25]);
        mixer
            .add_source(Stem::Instrumental, SoundSource::Pcm(inst))
            .unwrap();
        mixer
            .add_source(Stem::Vocals, SoundSource::Pcm(vocals))
            .unwrap();
        mixer.set_stem_gain(Stem::Vocals, 0.5);

        let mut out = [0.0; 2];
        mixer.mix_stereo(&mut out).unwrap();

        assert_eq!(out, [0.625, 0.375]);
    }

    #[test]
    fn looped_sources_wrap_to_the_beginning() {
        let mut mixer = Mixer::new(48_000);
        let samples: Arc<[f32]> = Arc::from([1.0, 1.0, 0.25, 0.25]);
        mixer
            .add_looped_source(Stem::Sfx, SoundSource::Pcm(samples))
            .unwrap();
        let mut out = [0.0; 6];

        mixer.mix_stereo(&mut out).unwrap();

        assert_eq!(out, [1.0, 1.0, 0.25, 0.25, 1.0, 1.0]);
        assert_eq!(mixer.voice_count(), 1);
    }

    #[test]
    fn has_voice_tracks_voice_lifetime() {
        let mut mixer = Mixer::new(48_000);
        let samples: Arc<[f32]> = Arc::from([1.0, 1.0, 0.25, 0.25]);
        let voice = mixer
            .add_source(Stem::Sfx, SoundSource::Pcm(samples))
            .unwrap();

        assert!(mixer.has_voice(voice));
        assert!(mixer.stop_voice(voice));
        assert!(!mixer.has_voice(voice));
    }

    #[test]
    fn pause_preserves_cursor_and_source_position() {
        let mut mixer = Mixer::new(48_000);
        let samples: Arc<[f32]> = Arc::from([1.0, 1.0, 0.25, 0.25]);
        mixer
            .add_source(Stem::Instrumental, SoundSource::Pcm(samples))
            .unwrap();
        mixer.pause();
        let mut out = [99.0; 2];

        mixer.mix_stereo(&mut out).unwrap();
        assert_eq!(out, [0.0, 0.0]);
        assert_eq!(mixer.sample_cursor(), Samples(0));

        mixer.resume();
        mixer.mix_stereo(&mut out).unwrap();
        assert_eq!(out, [1.0, 1.0]);
        assert_eq!(mixer.sample_cursor(), Samples(1));
    }

    #[test]
    fn paused_mixer_still_plays_sfx_without_advancing_song_cursor() {
        let mut mixer = Mixer::new(48_000);
        let samples: Arc<[f32]> = Arc::from([0.75, -0.75, 0.25, -0.25]);
        mixer
            .add_source(Stem::Sfx, SoundSource::Pcm(samples))
            .unwrap();
        mixer.pause();
        let mut out = [0.0; 2];

        mixer.mix_stereo(&mut out).unwrap();
        assert_eq!(out, [0.75, -0.75]);
        assert_eq!(mixer.sample_cursor(), Samples(0));

        mixer.mix_stereo(&mut out).unwrap();
        assert_eq!(out, [0.25, -0.25]);
        assert_eq!(mixer.sample_cursor(), Samples(0));
    }

    #[test]
    fn seek_moves_cursor_and_streaming_source() {
        let seeks = Arc::new(Mutex::new(Vec::new()));
        let decoder =
            TestDecoder::new(48_000, 2, vec![0.0, 0.0, 1.0, 1.0, 2.0, 2.0], seeks.clone());
        let mut mixer = Mixer::new(48_000);
        mixer
            .add_source(
                Stem::Instrumental,
                SoundSource::Streaming(Box::new(decoder)),
            )
            .unwrap();

        mixer.seek(Samples(2)).unwrap();
        let mut out = [0.0; 2];
        mixer.mix_stereo(&mut out).unwrap();

        assert_eq!(out, [2.0, 2.0]);
        assert_eq!(mixer.sample_cursor(), Samples(3));
        assert_eq!(*seeks.lock().unwrap(), vec![Samples(2)]);
    }

    #[test]
    fn seek_stems_preserves_sfx_source_position() {
        let mut mixer = Mixer::new(48_000);
        let song: Arc<[f32]> = Arc::from([1.0, 1.0, 2.0, 2.0]);
        let sfx: Arc<[f32]> = Arc::from([0.25, -0.25, 0.75, -0.75]);
        mixer
            .add_source(Stem::Instrumental, SoundSource::Pcm(song))
            .unwrap();
        mixer.add_source(Stem::Sfx, SoundSource::Pcm(sfx)).unwrap();

        let mut out = [0.0; 2];
        mixer.mix_stereo(&mut out).unwrap();
        assert_eq!(out, [1.25, 0.75]);

        mixer
            .seek_stems(Samples(0), &[Stem::Instrumental, Stem::Vocals])
            .unwrap();
        mixer.pause();
        mixer.mix_stereo(&mut out).unwrap();

        assert_eq!(out, [0.75, -0.75]);
        assert_eq!(mixer.sample_cursor(), Samples(0));
    }

    #[test]
    fn streaming_sources_resample_linearly() {
        let decoder = TestDecoder::new(24_000, 1, vec![0.0, 1.0, 2.0], Arc::default());
        let mut mixer = Mixer::new(48_000);
        mixer
            .add_source(
                Stem::Instrumental,
                SoundSource::Streaming(Box::new(decoder)),
            )
            .unwrap();
        let mut out = [0.0; 6];

        mixer.mix_stereo(&mut out).unwrap();

        assert_eq!(out, [0.0, 0.0, 0.5, 0.5, 1.0, 1.0]);
    }

    struct TestDecoder {
        sample_rate: u32,
        channels: u16,
        data: Vec<f32>,
        cursor: usize,
        seeks: Arc<Mutex<Vec<Samples>>>,
    }

    impl TestDecoder {
        fn new(
            sample_rate: u32,
            channels: u16,
            data: Vec<f32>,
            seeks: Arc<Mutex<Vec<Samples>>>,
        ) -> Self {
            Self {
                sample_rate,
                channels,
                data,
                cursor: 0,
                seeks,
            }
        }
    }

    impl Decoder for TestDecoder {
        fn sample_rate(&self) -> u32 {
            self.sample_rate
        }

        fn channels(&self) -> u16 {
            self.channels
        }

        fn read(&mut self, out: &mut [f32]) -> AudioResult<usize> {
            let channels = usize::from(self.channels);
            let requested_frames = out.len() / channels;
            let total_frames = self.data.len() / channels;
            let frames = requested_frames.min(total_frames.saturating_sub(self.cursor));
            let src = self.cursor * channels;
            let dst = frames * channels;
            out[..dst].copy_from_slice(&self.data[src..src + dst]);
            self.cursor += frames;
            Ok(frames)
        }

        fn seek(&mut self, position: Samples) -> AudioResult<()> {
            let channels = usize::from(self.channels);
            let frame = position.0.max(0) as usize;
            if frame > self.data.len() / channels {
                return Err(AudioError::SeekRange(format!("frame {}", position.0)));
            }
            self.cursor = frame;
            self.seeks.lock().unwrap().push(position);
            Ok(())
        }
    }
}
