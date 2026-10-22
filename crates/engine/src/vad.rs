//! Streaming Silero VAD (through sherpa-onnx, so the app links a single ONNX runtime):
//! cuts a dictation into speech segments as it is recorded, dropping the silence.

use std::path::Path;

use anyhow::{anyhow, ensure, Result};
use barnaby_core::SAMPLE_RATE;
use sherpa_rs::silero_vad::{SileroVad, SileroVadConfig};

/// Kept around each speech segment so word edges are not clipped.
const PADDING: usize = SAMPLE_RATE as usize / 5;
const WINDOW: usize = 512;

/// A speech segment: start in samples from the beginning of the stream, and its samples.
pub type Segment = (usize, Vec<f32>);

pub struct VadStream {
    vad: SileroVad,
    /// Everything pushed so far, to cut padded segments from.
    audio: Vec<f32>,
    /// Samples already given to Silero.
    fed: usize,
    /// End of the last returned segment: padding never overlaps it.
    emitted: usize,
}

impl VadStream {
    pub fn new(model: &Path) -> Result<VadStream> {
        // sherpa-onnx does not report a missing model, it aborts later.
        ensure!(model.is_file(), "VAD model not found");
        let config = SileroVadConfig {
            model: model.to_str().ok_or_else(|| anyhow!("non UTF-8 model path"))?.to_owned(),
            // A sentence-level pause ends a segment; shorter ones keep context for the model.
            min_silence_duration: 0.5,
            min_speech_duration: 0.25,
            // Past this, sherpa-onnx cuts at the next short pause, so segments stay quick to transcribe.
            max_speech_duration: 20.0,
            window_size: WINDOW as i32,
            ..Default::default()
        };
        let vad = SileroVad::new(config, 60.0).map_err(|e| anyhow!("VAD load failed: {e}"))?;
        Ok(VadStream { vad, audio: Vec::new(), fed: 0, emitted: 0 })
    }

    /// Appends 16 kHz mono samples; returns the speech segments finished so far.
    pub fn push(&mut self, samples: &[f32]) -> Vec<Segment> {
        self.audio.extend_from_slice(samples);
        // sherpa-onnx decides speech/silence once per call, so feed it one window at a time.
        while self.audio.len() - self.fed >= WINDOW {
            self.vad.accept_waveform(self.audio[self.fed..self.fed + WINDOW].to_vec());
            self.fed += WINDOW;
        }
        self.drain()
    }

    /// End of the recording: the segments still open.
    pub fn finish(mut self) -> Vec<Segment> {
        if self.fed < self.audio.len() {
            self.vad.accept_waveform(self.audio[self.fed..].to_vec());
            self.fed = self.audio.len();
        }
        self.vad.flush();
        self.drain()
    }

    fn drain(&mut self) -> Vec<Segment> {
        let mut out = Vec::new();
        while !self.vad.is_empty() {
            let seg = self.vad.front();
            self.vad.pop();
            let start = usize::try_from(seg.start).unwrap_or(0);
            let (s, e) = pad(start, start + seg.samples.len(), self.emitted, self.audio.len());
            if s < e {
                out.push((s, self.audio[s..e].to_vec()));
                self.emitted = e;
            }
        }
        out
    }
}

/// Widens `start..end` by [`PADDING`] within `floor..len`.
fn pad(start: usize, end: usize, floor: usize, len: usize) -> (usize, usize) {
    (start.saturating_sub(PADDING).max(floor), (end + PADDING).min(len))
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn padding_stays_in_bounds_and_never_overlaps() {
        assert_eq!(pad(10_000, 20_000, 0, 100_000), (10_000 - PADDING, 20_000 + PADDING));
        assert_eq!(pad(100, 20_000, 0, 20_100), (0, 20_100));
        assert_eq!(pad(10_000, 20_000, 9_900, 100_000), (9_900, 20_000 + PADDING));
    }

    /// jfk.wav from the test model directory, or downloaded once.
    fn jfk() -> Vec<f32> {
        let path = crate::models::tests::models_dir().join("jfk.wav");
        if !path.is_file() {
            let mut resp = ureq::get("https://raw.githubusercontent.com/ggml-org/whisper.cpp/master/samples/jfk.wav").call().unwrap();
            std::fs::write(&path, resp.body_mut().read_to_vec().unwrap()).unwrap();
        }
        hound::WavReader::open(path).unwrap().into_samples::<i16>().map(|s| f32::from(s.unwrap()) / 32768.0).collect()
    }

    /// Downloads `silero-vad`; uses whisper.cpp's 11 s speech sample.
    #[test]
    #[ignore]
    fn silero_streams_speech_and_drops_tone() {
        let model = crate::models::tests::ensure("silero-vad");
        let speech = jfk();
        let mut vad = VadStream::new(&model).unwrap();
        let mut segments: Vec<Segment> = speech.chunks(800).flat_map(|c| vad.push(c)).collect();
        let streamed = segments.len();
        segments.extend(vad.finish());
        assert!(streamed > 0, "segments come out while recording");
        assert!(segments.windows(2).all(|w| w[0].0 + w[0].1.len() <= w[1].0), "in order, no overlap");
        let kept: usize = segments.iter().map(|s| s.1.len()).sum();
        assert!(kept > speech.len() * 3 / 4 && kept <= speech.len(), "{kept} of {}", speech.len());

        let mut vad = VadStream::new(&model).unwrap();
        let hum: Vec<f32> = (0..48_000).map(|i| 0.01 * (i as f32 * 0.05).sin()).collect();
        let mut kept: usize = vad.push(&hum).iter().map(|s| s.1.len()).sum();
        kept += vad.finish().iter().map(|s| s.1.len()).sum::<usize>();
        assert!(kept < hum.len() / 2);
    }
}
