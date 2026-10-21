//! Shared domain types. JSON shapes follow `docs/SPEC.md` section 14 and are
//! mirrored by hand in `ui/src/lib/api.ts`: change both together.

use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};
use uuid::Uuid;

pub const SCHEMA_VERSION: &str = "1.0";
/// All audio handed to providers: 16 kHz, mono, f32 in [-1, 1].
pub const SAMPLE_RATE: u32 = 16_000;

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum MeetingState {
    Recording,
    Captured,
    Queued,
    Processing,
    NeedsReview,
    Reviewed,
    Sending,
    Sent,
    Failed,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum ProcessingMode {
    Local,
    Api,
    Hybrid,
}

/// Pipeline steps, in execution order. Each one's output is persisted.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum Step {
    Transcribe,
    Diarize,
    Identify,
    Review,
    Structure,
}

impl Step {
    pub const ALL: [Step; 5] = [Step::Transcribe, Step::Diarize, Step::Identify, Step::Review, Step::Structure];
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct Participant {
    pub id: String,
    pub name: String,
    pub email: Option<String>,
    pub confirmed: bool,
}

/// Row-level view of a meeting, as listed on the home screen.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct Meeting {
    pub id: Uuid,
    pub title: String,
    pub started_at: DateTime<Utc>,
    pub ended_at: Option<DateTime<Utc>>,
    pub language: Option<String>,
    pub state: MeetingState,
    /// Failure cause (error code / message without content), set when `state == Failed`.
    pub error: Option<String>,
    pub mode: ProcessingMode,
    pub review_items_total: u32,
    pub review_items_resolved: u32,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct Segment {
    pub id: String,
    pub start: f64,
    pub end: f64,
    pub speaker_id: Option<String>,
    pub text: String,
    pub confidence: f32,
    pub reviewed: bool,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct Note {
    pub id: String,
    /// Seconds from meeting start.
    pub at: f64,
    pub text: String,
    /// "manual" | "marker"
    pub source: String,
}

#[derive(Debug, Clone, Default, PartialEq, Serialize, Deserialize)]
pub struct ActionItem {
    pub owner: Option<String>,
    pub task: String,
    pub due: Option<String>,
}

#[derive(Debug, Clone, Default, PartialEq, Serialize, Deserialize)]
pub struct Summary {
    pub overview: String,
    pub decisions: Vec<String>,
    pub action_items: Vec<ActionItem>,
    pub open_questions: Vec<String>,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct MeetingInfo {
    pub id: Uuid,
    pub title: String,
    pub started_at: DateTime<Utc>,
    pub ended_at: Option<DateTime<Utc>>,
    pub language: Option<String>,
    pub participants: Vec<Participant>,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct Processing {
    pub mode: ProcessingMode,
    /// e.g. {"transcription": "whisper.cpp", "diarization": "sherpa-onnx"}
    pub providers: std::collections::BTreeMap<String, String>,
    pub review_items_total: u32,
    pub review_items_resolved: u32,
}

/// The exported meeting document (spec section 14).
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct MeetingDoc {
    pub schema_version: String,
    pub meeting: MeetingInfo,
    pub segments: Vec<Segment>,
    pub notes: Vec<Note>,
    pub summary: Option<Summary>,
    pub processing: Processing,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum ReviewKind {
    LowConfidence,
    Overlap,
    UnattributedSpeaker,
    UncertainTerm,
    Inaudible,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(tag = "action", rename_all = "snake_case")]
pub enum Resolution {
    Validate,
    Correct { text: String },
    Reassign { speaker_id: String },
    MarkInaudible,
    Ignore,
}

/// A "zone d'ombre" (spec section 13). The audio excerpt is `start..end` of the meeting audio.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct ReviewItem {
    pub id: String,
    pub meeting_id: Uuid,
    pub segment_id: Option<String>,
    pub kind: ReviewKind,
    pub start: f64,
    pub end: f64,
    pub proposed_text: String,
    pub alternatives: Vec<String>,
    /// Question shown to the user, as an i18n key (e.g. "review.q.low_confidence").
    pub question: String,
    pub resolution: Option<Resolution>,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct Dictation {
    pub id: Uuid,
    pub created_at: DateTime<Utc>,
    pub text: String,
    pub language: Option<String>,
    pub duration_ms: u64,
    pub injected_into: Option<String>,
    pub provider: String,
}

/// Dictation export (spec section 14).
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct DictationDoc {
    pub schema_version: String,
    pub dictation: Dictation,
}

// ---------- Settings ----------

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum DictationMode {
    Hold,
    Toggle,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum AudioRetention {
    /// Delete meeting audio when the review is validated (default).
    OnReview,
    SevenDays,
    Forever,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(default)]
pub struct DictationSettings {
    pub mode: DictationMode,
    /// Model id from the model catalog.
    pub model: String,
    pub paste_delay_before_ms: u64,
    pub paste_delay_after_ms: u64,
    pub keep_history: bool,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(default)]
pub struct MeetingSettings {
    pub model: String,
    pub confidence_threshold: f32,
    pub retention: AudioRetention,
}

/// Accelerators in Tauri global-shortcut syntax, e.g. "Ctrl+Alt+Space".
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(default)]
pub struct Shortcuts {
    pub dictation: String,
    pub meeting_toggle: String,
    pub marker: String,
    pub review_queue: String,
}

#[derive(Debug, Clone, Default, PartialEq, Serialize, Deserialize)]
#[serde(default)]
pub struct FolderDestination {
    pub enabled: bool,
    pub path: String,
}

/// The HMAC secret lives in the OS keyring, never here.
#[derive(Debug, Clone, Default, PartialEq, Serialize, Deserialize)]
#[serde(default)]
pub struct WebhookDestination {
    pub enabled: bool,
    pub url: String,
}

#[derive(Debug, Clone, Default, PartialEq, Serialize, Deserialize)]
#[serde(default)]
pub struct Destinations {
    pub folder: FolderDestination,
    pub webhook: WebhookDestination,
}

/// OpenAI-compatible endpoint used for structuring (Ollama in local mode).
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(default)]
pub struct LlmSettings {
    pub enabled: bool,
    pub base_url: String,
    pub model: String,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(default)]
pub struct Settings {
    /// "fr" | "en"
    pub ui_language: String,
    /// "auto" | "fr" | "en"
    pub transcription_language: String,
    pub processing_mode: ProcessingMode,
    pub dictation: DictationSettings,
    pub meeting: MeetingSettings,
    pub shortcuts: Shortcuts,
    pub destinations: Destinations,
    pub llm: LlmSettings,
    pub consent_acknowledged: bool,
    pub notification_sound: bool,
}

impl Default for DictationSettings {
    fn default() -> Self {
        Self { mode: DictationMode::Hold, model: "whisper-base".into(), paste_delay_before_ms: 50, paste_delay_after_ms: 300, keep_history: true }
    }
}

impl Default for MeetingSettings {
    fn default() -> Self {
        Self { model: "whisper-small".into(), confidence_threshold: 0.6, retention: AudioRetention::OnReview }
    }
}

impl Default for Shortcuts {
    fn default() -> Self {
        Self {
            dictation: "Ctrl+Alt+Space".into(),
            meeting_toggle: "Ctrl+Shift+R".into(),
            marker: "Ctrl+Shift+M".into(),
            review_queue: "Ctrl+Shift+L".into(),
        }
    }
}

impl Default for LlmSettings {
    fn default() -> Self {
        Self { enabled: false, base_url: "http://localhost:11434/v1".into(), model: "llama3.1:8b".into() }
    }
}

impl Default for Settings {
    fn default() -> Self {
        Self {
            ui_language: "fr".into(),
            transcription_language: "auto".into(),
            processing_mode: ProcessingMode::Local,
            dictation: DictationSettings::default(),
            meeting: MeetingSettings::default(),
            shortcuts: Shortcuts::default(),
            destinations: Destinations::default(),
            llm: LlmSettings::default(),
            consent_acknowledged: false,
            notification_sound: false,
        }
    }
}
