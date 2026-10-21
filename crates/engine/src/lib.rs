//! Native side of Barnaby: audio capture, VAD, whisper.cpp and sherpa-onnx providers,
//! and the local model catalog. Knows nothing of Tauri or SQLite.

pub mod capture;
pub mod diarize;
mod dsp;
mod input;
pub mod mic;
pub mod models;
pub mod vad;
pub mod whisper;

#[cfg(test)]
mod e2e;

pub use capture::Recorder;
pub use diarize::SherpaDiarizer;
pub use mic::DictationRecorder;
pub use models::{ModelEntry, ModelKind};
pub use vad::Vad;
pub use whisper::WhisperTranscriber;

// The app keeps recorders and providers in shared state across threads.
const _: () = {
    const fn send_sync<T: Send + Sync>() {}
    const fn send<T: Send>() {}
    send::<Recorder>();
    send::<DictationRecorder>();
    send_sync::<WhisperTranscriber>();
    send_sync::<SherpaDiarizer>();
    send_sync::<Vad>();
};
