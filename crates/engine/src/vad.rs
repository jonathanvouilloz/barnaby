//! Silero VAD (through sherpa-onnx, so the app links a single ONNX runtime) to drop
//! silence from dictation before transcription.

use std::path::Path;
use std::sync::Mutex;

use anyhow::{anyhow, ensure, Result};
use barnaby_core::SAMPLE_RATE;
use sherpa_rs::silero_vad::{SileroVad, SileroVadConfig};

use crate::dsp::rms;

/// Kept around each voiced chunk so word edges are not clipped.
const PADDING: usize = SAMPLE_RATE as usize / 5;
const WINDOW: usize = 512;
/// Below this overall RMS (about -60 dBFS) the buffer is silence; skips the model.
const SILENCE_RMS: f32 = 1e-3;

pub struct Vad {
    inner: Mutex<SileroVad>,
}

impl Vad {
    pub fn load(model: &Path) -> Result<Vad> {
        // sherpa-onnx does not report a missing model, it aborts later.
        ensure!(model.is_file(), "VAD model not found");
        let config = SileroVadConfig {
            model: model.to_str().ok_or_else(|| anyhow!("non UTF-8 model path"))?.to_owned(),
            min_silence_duration: 0.3,
            min_speech_duration: 0.25,
            max_speech_duration: 30.0,
            window_size: WINDOW as i32,
            ..Default::default()
        };
        let vad = SileroVad::new(config, 60.0).map_err(|e| anyhow!("VAD load failed: {e}"))?;
        Ok(Vad { inner: Mutex::new(vad) })
    }

    /// Voiced parts of 16 kHz mono `samples`, padded; empty when there is no speech.
    pub fn trim_silence(&self, samples: &[f32]) -> Vec<f32> {
        if is_silence(samples) {
            return Vec::new();
        }
        let mut vad = self.inner.lock().unwrap_or_else(|e| e.into_inner());
        vad.clear();
        // sherpa-onnx decides speech/silence once per call, so feed it one window at a time.
        for window in samples.chunks(WINDOW) {
            vad.accept_waveform(window.to_vec());
        }
        vad.flush();
        let mut spans = Vec::new();
        while !vad.is_empty() {
            let seg = vad.front();
            let start = usize::try_from(seg.start).unwrap_or(0);
            spans.push((start, start + seg.samples.len()));
            vad.pop();
        }
        vad.clear();
        splice(samples, &spans)
    }
}

fn is_silence(samples: &[f32]) -> bool {
    rms(samples) < SILENCE_RMS
}

/// Concatenates `spans` of `samples` widened by [`PADDING`], merging overlaps.
fn splice(samples: &[f32], spans: &[(usize, usize)]) -> Vec<f32> {
    let mut merged: Vec<(usize, usize)> = Vec::new();
    for &(s, e) in spans {
        let (s, e) = (s.saturating_sub(PADDING), (e + PADDING).min(samples.len()));
        match merged.last_mut() {
            Some(last) if s <= last.1 => last.1 = last.1.max(e),
            _ => merged.push((s, e)),
        }
    }
    merged.into_iter().filter(|(s, e)| s < e).flat_map(|(s, e)| samples[s..e].iter().copied()).collect()
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn splice_pads_and_merges() {
        let samples: Vec<f32> = (0..32_000).map(|i| i as f32).collect();
        let out = splice(&samples, &[(4_000, 5_000), (5_500, 6_000), (20_000, 40_000)]);
        assert_eq!(out.len(), (6_000 + PADDING - (4_000 - PADDING)) + (32_000 - (20_000 - PADDING)));
        assert_eq!(out[0], (4_000 - PADDING) as f32);
        assert!(splice(&samples, &[]).is_empty());
    }

    /// The energy gate that short-circuits Silero on silent input.
    #[test]
    fn silence_gate() {
        assert!(is_silence(&vec![0.0; 32_000]));
        assert!(is_silence(&[]));
        assert!(is_silence(&vec![5e-4; 16_000]));
        assert!(!is_silence(&vec![0.05; 16_000]));
    }

    /// Downloads `silero-vad` and whisper.cpp's 11 s speech sample.
    #[test]
    #[ignore]
    fn silero_keeps_speech_and_drops_tone() {
        let vad = Vad::load(&crate::models::tests::ensure("silero-vad")).unwrap();
        let mut resp = ureq::get("https://raw.githubusercontent.com/ggml-org/whisper.cpp/master/samples/jfk.wav").call().unwrap();
        let wav = resp.body_mut().read_to_vec().unwrap();
        let speech: Vec<f32> = hound::WavReader::new(std::io::Cursor::new(wav)).unwrap().into_samples::<i16>().map(|s| f32::from(s.unwrap()) / 32768.0).collect();
        let kept = vad.trim_silence(&speech).len();
        assert!(kept > speech.len() * 3 / 4 && kept < speech.len(), "{kept} of {}", speech.len());

        let hum: Vec<f32> = (0..48_000).map(|i| 0.01 * (i as f32 * 0.05).sin()).collect();
        assert!(vad.trim_silence(&hum).len() < hum.len() / 2);
    }
}
