//! whisper.cpp transcription. One context per loaded model, one state per call,
//! so a dictation and a meeting can share a model without locking.

use std::path::Path;

use anyhow::{anyhow, Result};
use barnaby_core::{RawSegment, Transcriber, Transcript};
use whisper_rs::{FullParams, SamplingStrategy, WhisperContext, WhisperContextParameters};

pub struct WhisperTranscriber {
    ctx: WhisperContext,
    threads: i32,
}

impl WhisperTranscriber {
    /// `use_gpu` only matters in builds with the `cuda` or `vulkan` feature.
    pub fn load(model_path: &Path, use_gpu: bool) -> Result<WhisperTranscriber> {
        whisper_rs::install_logging_hooks();
        let mut params = WhisperContextParameters::default();
        params.use_gpu(use_gpu);
        let ctx = WhisperContext::new_with_params(model_path, params).map_err(|e| anyhow!("whisper model load failed: {e}"))?;
        let threads = std::thread::available_parallelism().map_or(4, |n| n.get()) as i32;
        Ok(WhisperTranscriber { ctx, threads })
    }
}

impl Transcriber for WhisperTranscriber {
    fn name(&self) -> &str {
        "whisper.cpp"
    }

    fn transcribe(&self, samples: &[f32], language: Option<&str>) -> Result<Transcript> {
        let mut state = self.ctx.create_state().map_err(|e| anyhow!("whisper state: {e}"))?;
        let mut params = FullParams::new(SamplingStrategy::Greedy { best_of: 1 });
        params.set_n_threads(self.threads);
        params.set_language(Some(language.unwrap_or("auto")));
        params.set_print_special(false);
        params.set_print_progress(false);
        params.set_print_realtime(false);
        params.set_print_timestamps(false);
        state.full(params, samples).map_err(|e| anyhow!("whisper inference: {e}"))?;

        // Ids from end-of-text upward are special tokens (timestamps, language, task).
        let eot = self.ctx.token_eot();
        let segments = state
            .as_iter()
            .map(|seg| {
                let probs: Vec<f32> = (0..seg.n_tokens())
                    .filter_map(|i| seg.get_token(i))
                    .filter(|t| t.token_id() < eot)
                    .map(|t| t.token_probability())
                    .collect();
                RawSegment {
                    start: seg.start_timestamp() as f64 / 100.0,
                    end: seg.end_timestamp() as f64 / 100.0,
                    text: seg.to_str_lossy().map(|s| s.trim().to_owned()).unwrap_or_default(),
                    confidence: if probs.is_empty() { 0.0 } else { probs.iter().sum::<f32>() / probs.len() as f32 },
                }
            })
            .collect();
        let language = whisper_rs::get_lang_str(state.full_lang_id_from_state()).unwrap_or("").to_owned();
        Ok(Transcript { language, segments })
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    /// Downloads `whisper-tiny` (78 MB) on first run; checks the call path, not accuracy.
    #[test]
    #[ignore]
    fn tiny_transcribes_a_tone() {
        let path = crate::models::tests::ensure("whisper-tiny");
        let t = WhisperTranscriber::load(&path, false).unwrap();
        let audio: Vec<f32> = (0..16_000 * 3).map(|i| 0.1 * (i as f32 * 0.1).sin()).collect();
        let out = t.transcribe(&audio, Some("fr")).unwrap();
        assert_eq!(out.language, "fr");
        for s in &out.segments {
            assert!(s.start <= s.end && (0.0..=1.0).contains(&s.confidence), "{s:?}");
        }
        let auto = t.transcribe(&audio, None).unwrap();
        assert!(!auto.language.is_empty());
    }
}
