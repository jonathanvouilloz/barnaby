//! Provider contracts (spec section 6). Implemented by `barnaby-engine` (local)
//! and, later, API clients. All calls are blocking; callers run them off the UI thread.

use crate::model::{MeetingDoc, Summary};
use serde::{Deserialize, Serialize};

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
}

pub trait Diarizer: Send + Sync {
    fn name(&self) -> &str;
    fn diarize(&self, samples: &[f32]) -> anyhow::Result<Vec<SpeakerTurn>>;
}

pub trait Structurer: Send + Sync {
    fn name(&self) -> &str;
    fn structure(&self, doc: &MeetingDoc) -> anyhow::Result<Summary>;
}

pub trait Destination: Send + Sync {
    fn name(&self) -> &str;
    fn send(&self, doc: &MeetingDoc) -> anyhow::Result<()>;
}
