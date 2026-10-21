//! NVIDIA Parakeet TDT (int8, through sherpa-onnx): much faster than whisper on CPU,
//! so it is the dictation model. 25 European languages, punctuated output.

use std::path::Path;
use std::sync::Mutex;

use anyhow::{anyhow, ensure, Result};
use barnaby_core::{RawSegment, Transcriber, Transcript, SAMPLE_RATE};
use sherpa_rs::transducer::{TransducerConfig, TransducerRecognizer};

pub struct ParakeetTranscriber {
    recognizer: Mutex<TransducerRecognizer>,
}

impl ParakeetTranscriber {
    /// `dir`: the unpacked `sherpa-onnx-nemo-parakeet-tdt-*-int8` archive.
    pub fn load(dir: &Path) -> Result<ParakeetTranscriber> {
        let file = |name: &str| -> Result<String> {
            let path = dir.join(name);
            // sherpa-onnx does not report a missing file, it aborts.
            ensure!(path.is_file(), "parakeet: {name} not found");
            path.to_str().map(str::to_owned).ok_or_else(|| anyhow!("non UTF-8 model path"))
        };
        let config = TransducerConfig {
            encoder: file("encoder.int8.onnx")?,
            decoder: file("decoder.int8.onnx")?,
            joiner: file("joiner.int8.onnx")?,
            tokens: file("tokens.txt")?,
            model_type: "nemo_transducer".into(),
            decoding_method: "greedy_search".into(),
            sample_rate: SAMPLE_RATE as i32,
            feature_dim: 80,
            num_threads: std::thread::available_parallelism().map_or(4, |n| n.get()) as i32,
            ..Default::default()
        };
        let recognizer = TransducerRecognizer::new(config).map_err(|e| anyhow!("parakeet load failed: {e}"))?;
        Ok(ParakeetTranscriber { recognizer: Mutex::new(recognizer) })
    }
}

impl Transcriber for ParakeetTranscriber {
    fn name(&self) -> &str {
        "parakeet"
    }

    /// The model detects the language itself but does not report it, nor token probabilities.
    fn transcribe(&self, samples: &[f32], language: Option<&str>) -> Result<Transcript> {
        let language = language.unwrap_or("auto").to_owned();
        if samples.is_empty() {
            return Ok(Transcript { language, segments: Vec::new() });
        }
        let text = self.recognizer.lock().unwrap_or_else(|e| e.into_inner()).transcribe(SAMPLE_RATE, samples);
        let text = text.trim();
        let end = samples.len() as f64 / f64::from(SAMPLE_RATE);
        let segments = if text.is_empty() { Vec::new() } else { vec![RawSegment { start: 0.0, end, text: text.to_owned(), confidence: 1.0 }] };
        Ok(Transcript { language, segments })
    }
}

#[cfg(test)]
pub(crate) mod tests {
    use super::*;

    /// fr.wav (22.05 kHz) from the Parakeet archive, resampled to 16 kHz.
    pub fn french_sample(dir: &Path) -> Vec<f32> {
        let mut reader = hound::WavReader::open(dir.join("test_wavs/fr.wav")).unwrap();
        let rate = reader.spec().sample_rate;
        let raw: Vec<f32> = reader.samples::<i16>().map(|s| f32::from(s.unwrap()) / 32768.0).collect();
        crate::dsp::Resampler::new(rate).process(&raw)
    }

    pub fn model_dir() -> std::path::PathBuf {
        let tokens = crate::models::tests::ensure("parakeet-v3");
        tokens.parent().unwrap().to_owned()
    }

    /// Downloads `parakeet-v3` (487 MB) unless `BARNABY_MODELS_DIR` already has it.
    #[test]
    #[ignore]
    fn transcribes_french() {
        let dir = model_dir();
        let t = ParakeetTranscriber::load(&dir).unwrap();
        let out = t.transcribe(&french_sample(&dir), Some("fr")).unwrap();
        assert_eq!(out.language, "fr");
        assert_eq!(out.segments.len(), 1);
        assert!(out.segments[0].text.split_whitespace().count() > 5, "{:?}", out.segments[0].text);
        assert!(t.transcribe(&[], None).unwrap().segments.is_empty());
    }
}
