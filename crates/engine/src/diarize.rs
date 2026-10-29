//! sherpa-onnx speaker diarization: pyannote segmentation + speaker embeddings,
//! clustered into the expected speaker count, or by threshold when it is unknown.

use std::path::{Path, PathBuf};

use anyhow::{anyhow, ensure, Result};
use barnaby_core::{Diarizer, ProgressFn, SpeakerTurn};
use sherpa_rs::diarize::{Diarize, DiarizeConfig};

/// Cosine-distance threshold of sherpa-onnx's fast clustering, used when the speaker count
/// is unknown: lower splits into more speakers. sherpa-onnx's default 0.5 split one person
/// into several on poor meeting mics; 0.7 merges near voices instead.
const CLUSTER_THRESHOLD: f32 = 0.7;

/// The speaker count changes per meeting and sherpa-rs fixes it at load, so each call
/// loads the two models (a few tens of MB, well under the diarization time).
pub struct SherpaDiarizer {
    segmentation_model: PathBuf,
    embedding_model: PathBuf,
}

impl SherpaDiarizer {
    pub fn load(segmentation_model: &Path, embedding_model: &Path) -> Result<SherpaDiarizer> {
        ensure!(segmentation_model.is_file() && embedding_model.is_file(), "diarization model not found");
        Ok(SherpaDiarizer { segmentation_model: segmentation_model.into(), embedding_model: embedding_model.into() })
    }
}

impl Diarizer for SherpaDiarizer {
    fn name(&self) -> &str {
        "sherpa-onnx"
    }

    fn diarize(&self, samples: &[f32], speakers: Option<u32>) -> Result<Vec<SpeakerTurn>> {
        self.diarize_with_progress(samples, speakers, &(std::sync::Arc::new(|_| {}) as ProgressFn))
    }

    fn diarize_with_progress(&self, samples: &[f32], speakers: Option<u32>, progress: &ProgressFn) -> Result<Vec<SpeakerTurn>> {
        let config = DiarizeConfig {
            // -1: unknown, clustered by threshold.
            num_clusters: Some(speakers.map_or(-1, |n| n as i32)),
            threshold: Some(CLUSTER_THRESHOLD),
            // sherpa-onnx's recommended values: drop blips, bridge short pauses.
            min_duration_on: Some(0.3),
            min_duration_off: Some(0.5),
            ..Default::default()
        };
        let mut d = Diarize::new(&self.segmentation_model, &self.embedding_model, config)
            .map_err(|e| anyhow!("diarization load failed: {e}"))?;
        let p = progress.clone();
        // sherpa-onnx ignores the callback's return value; 0 is what its examples return.
        let on_chunk: Box<dyn Fn(i32, i32) -> i32 + Send> = Box::new(move |done, total| {
            if total > 0 {
                p(done as f32 / total as f32);
            }
            0
        });
        match d.compute(samples.to_vec(), Some(on_chunk)) {
            Ok(segments) => Ok(segments
                .into_iter()
                .map(|s| SpeakerTurn { start: f64::from(s.start), end: f64::from(s.end), speaker: s.speaker.max(0) as u32 })
                .collect()),
            // sherpa-rs reports "no speech" as an error.
            Err(e) if e.to_string().starts_with("No segments found") => Ok(Vec::new()),
            Err(e) => Err(anyhow!("diarization failed: {e}")),
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::models::tests::ensure;

    /// Downloads the two diarization models on first run.
    #[test]
    #[ignore]
    fn loads_and_handles_silence_and_tone() {
        let d = SherpaDiarizer::load(&ensure("pyannote-segmentation-3-0"), &ensure("wespeaker-voxceleb-resnet34-lm")).unwrap();
        assert!(d.diarize(&vec![0.0; 16_000 * 5], None).unwrap().is_empty());
        let tone: Vec<f32> = (0..16_000 * 5).map(|i| 0.1 * (i as f32 * 0.1).sin()).collect();
        for t in d.diarize(&tone, Some(2)).unwrap() {
            assert!(t.start <= t.end, "{t:?}");
        }
    }
}
