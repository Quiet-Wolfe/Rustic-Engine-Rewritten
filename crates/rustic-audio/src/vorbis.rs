//! Ogg/Vorbis decoder backed by the asset resolver's byte buffers.

use crate::error::{AudioError, AudioResult};
use crate::source::{Decoder, SoundSource};
use lewton::inside_ogg::OggStreamReader;
use rustic_core::time::Samples;
use std::io::Cursor;
use std::sync::Arc;

const I16_SCALE: f32 = 32768.0;

pub struct VorbisDecoder {
    bytes: Arc<[u8]>,
    reader: OggStreamReader<Cursor<Arc<[u8]>>>,
    sample_rate: u32,
    channels: u16,
    pending: Vec<f32>,
    pending_pos: usize,
}

impl VorbisDecoder {
    pub fn new(bytes: Arc<[u8]>) -> AudioResult<Self> {
        let reader = open_reader(bytes.clone())?;
        let sample_rate = reader.ident_hdr.audio_sample_rate;
        let channels = u16::from(reader.ident_hdr.audio_channels);
        Ok(Self {
            bytes,
            reader,
            sample_rate,
            channels,
            pending: Vec::new(),
            pending_pos: 0,
        })
    }

    fn reset(&mut self) -> AudioResult<()> {
        self.reader = open_reader(self.bytes.clone())?;
        self.pending.clear();
        self.pending_pos = 0;
        Ok(())
    }

    fn refill_pending(&mut self) -> AudioResult<bool> {
        let Some(samples) = self
            .reader
            .read_dec_packet_itl()
            .map_err(|err| AudioError::Decode(format!("vorbis packet: {err}")))?
        else {
            return Ok(false);
        };
        self.pending.clear();
        self.pending.extend(
            samples
                .into_iter()
                .map(|sample| f32::from(sample) / I16_SCALE),
        );
        self.pending_pos = 0;
        Ok(true)
    }
}

pub fn streaming_vorbis_source(bytes: Arc<[u8]>) -> AudioResult<SoundSource> {
    Ok(SoundSource::Streaming(Box::new(VorbisDecoder::new(bytes)?)))
}

impl Decoder for VorbisDecoder {
    fn sample_rate(&self) -> u32 {
        self.sample_rate
    }

    fn channels(&self) -> u16 {
        self.channels
    }

    fn read(&mut self, out: &mut [f32]) -> AudioResult<usize> {
        let channels = usize::from(self.channels).max(1);
        let writable_samples = out.len() - (out.len() % channels);
        let mut written = 0;

        while written < writable_samples {
            if self.pending_pos >= self.pending.len() && !self.refill_pending()? {
                break;
            }

            let pending_left = self.pending.len() - self.pending_pos;
            let out_left = writable_samples - written;
            let count = pending_left.min(out_left);
            out[written..written + count]
                .copy_from_slice(&self.pending[self.pending_pos..self.pending_pos + count]);
            self.pending_pos += count;
            written += count;
        }

        Ok(written / channels)
    }

    fn seek(&mut self, position: Samples) -> AudioResult<()> {
        let target = position.0.max(0) as usize;
        self.reset()?;

        let channels = usize::from(self.channels).max(1);
        let mut remaining = target;
        let mut scratch = vec![0.0; channels * 4096];
        while remaining > 0 {
            let frames = remaining.min(4096);
            let read = self.read(&mut scratch[..frames * channels])?;
            if read == 0 {
                break;
            }
            remaining -= read;
        }
        Ok(())
    }
}

fn open_reader(bytes: Arc<[u8]>) -> AudioResult<OggStreamReader<Cursor<Arc<[u8]>>>> {
    OggStreamReader::new(Cursor::new(bytes))
        .map_err(|err| AudioError::Decode(format!("vorbis headers: {err}")))
}

#[cfg(test)]
#[allow(clippy::unwrap_used, clippy::expect_used)]
mod tests {
    use super::*;
    use crate::{Mixer, Stem};
    use rustic_asset::{load_bytes, AssetPath, OverlayResolver};

    fn bopeebo_inst() -> Arc<[u8]> {
        let manifest_dir = std::path::Path::new(env!("CARGO_MANIFEST_DIR"));
        let workspace = manifest_dir.parent().unwrap().parent().unwrap();
        let source_root = workspace.join("assets/source");
        let resolver = OverlayResolver::new().with_baked_root(source_root);
        load_bytes(
            &resolver,
            &AssetPath::new("music/Bopeebo_Inst.ogg").unwrap(),
        )
        .unwrap()
    }

    #[test]
    fn decodes_imported_og_vorbis_stem() {
        let mut decoder = VorbisDecoder::new(bopeebo_inst()).unwrap();
        assert_eq!(decoder.sample_rate(), 44_100);
        assert_eq!(decoder.channels(), 2);

        let mut out = vec![0.0; usize::from(decoder.channels()) * 1024];
        let frames = decoder.read(&mut out).unwrap();

        assert!(frames > 0);
        assert!(out.iter().any(|sample| *sample != 0.0));
    }

    #[test]
    fn seek_restarts_decoding_from_requested_frame() {
        let mut decoder = VorbisDecoder::new(bopeebo_inst()).unwrap();
        let channels = usize::from(decoder.channels());
        let mut first = vec![0.0; channels * 16];
        decoder.read(&mut first).unwrap();

        decoder.seek(Samples(0)).unwrap();
        let mut after_seek = vec![0.0; channels * 16];
        decoder.read(&mut after_seek).unwrap();

        assert_eq!(first, after_seek);
    }

    #[test]
    fn mixer_accepts_imported_vorbis_stem() {
        let mut mixer = Mixer::new(48_000);
        mixer
            .add_source(
                Stem::Instrumental,
                streaming_vorbis_source(bopeebo_inst()).unwrap(),
            )
            .unwrap();
        let mut out = vec![0.0; 4096 * 2];

        let stats = mixer.mix_stereo(&mut out).unwrap();

        assert_eq!(stats.frames, 4096);
        assert_eq!(stats.sample_cursor, Samples(4096));
        assert!(out.iter().any(|sample| *sample != 0.0));
    }
}
