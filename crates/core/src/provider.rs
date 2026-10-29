//! Provider contracts (spec section 6). Implemented by `barnaby-engine` (local)
//! and, later, API clients. All calls are blocking; callers run them off the UI thread.

use std::sync::Arc;

use crate::model::{MeetingDoc, Summary};
use serde::{Deserialize, Serialize};

/// Share of the current step done, in [0, 1].
pub type ProgressFn = Arc<dyn Fn(f32) + Send + Sync>;

/// A transcribed span, times in seconds from the start of the audio.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct RawSegment {
    pub start: f64,
    pub end: f64,
    pub text: String,
    /// Mean token probability in [0, 1].
    pub confidence: f32,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct Transcript {
    /// Detected or forced language code ("fr", "en", ...).
    pub language: String,
    pub segments: Vec<RawSegment>,
}

/// "Who speaks when": anonymous speaker index per turn.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct SpeakerTurn {
    pub start: f64,
    pub end: f64,
    pub speaker: u32,
}

pub trait Transcriber: Send + Sync {
    /// Provider name written to `processing.providers` (e.g. "whisper.cpp").
    fn name(&self) -> &str;
    /// `samples`: 16 kHz mono f32. `language`: None = auto-detect.
    fn transcribe(&self, samples: &[f32], language: Option<&str>) -> anyhow::Result<Transcript>;
    /// Same, reporting progress; providers without intermediate steps keep the default.
    fn transcribe_with_progress(&self, samples: &[f32], language: Option<&str>, _progress: &ProgressFn) -> anyhow::Result<Transcript> {
        self.transcribe(samples, language)
    }
}

pub trait Diarizer: Send + Sync {
    fn name(&self) -> &str;
    /// `speakers`: how many voices to find; None = unknown.
    fn diarize(&self, samples: &[f32], speakers: Option<u32>) -> anyhow::Result<Vec<SpeakerTurn>>;
    fn diarize_with_progress(&self, samples: &[f32], speakers: Option<u32>, _progress: &ProgressFn) -> anyhow::Result<Vec<SpeakerTurn>> {
        self.diarize(samples, speakers)
    }
}

pub trait Structurer: Send + Sync {
    fn name(&self) -> &str;
    fn structure(&self, doc: &MeetingDoc) -> anyhow::Result<Summary>;
}

pub trait Destination: Send + Sync {
    fn name(&self) -> &str;
    fn send(&self, doc: &MeetingDoc) -> anyhow::Result<()>;
}

/// Short error code a provider attaches to a failure; the pipeline stores it on the meeting.
#[derive(Debug, thiserror::Error)]
#[error("{0}")]
pub struct Coded(pub &'static str);

/// The first `Coded` in `err`'s chain.
pub fn error_code(err: &anyhow::Error) -> Option<&'static str> {
    err.chain()
        .find_map(|e| e.downcast_ref::<Coded>())
        .map(|c| c.0)
}
