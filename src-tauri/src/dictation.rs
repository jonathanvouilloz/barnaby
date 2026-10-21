//! Dictation (spec 7): mic → VAD → whisper (ahead of meetings) → paste at cursor.

use std::{sync::Arc, thread, time::Duration};

use barnaby_core::{Dictation, Transcriber, SAMPLE_RATE};
use barnaby_engine::DictationRecorder;
use tauri::Emitter;
use tauri_plugin_notification::NotificationExt;

use crate::{
    inject,
    state::{lock, App},
};

fn status(app: &App, s: &str) {
    let _ = app.handle.emit("dictation-status", s);
    crate::tray::refresh(app);
}

pub fn is_recording(app: &App) -> bool {
    lock(&app.dictation).is_some()
}

pub fn start(app: &Arc<App>) {
    let mut slot = lock(&app.dictation);
    if slot.is_some() {
        return;
    }
    match DictationRecorder::start() {
        Ok(r) => *slot = Some(r),
        Err(e) => {
            log::warn!("dictation start: {e:#}");
            alert(app, if e.to_string() == "capture.permission_denied" { Alert::MicDenied } else { Alert::NoMic });
            return;
        }
    }
    drop(slot);
    // Spec 7.4: the mic is shared; keep the dictation out of a meeting's mic track.
    if let Some(m) = lock(&app.recording).as_ref() {
        m.recorder.pause_mic(true);
    }
    status(app, "recording");
}

pub fn stop(app: &Arc<App>) {
    let Some(rec) = lock(&app.dictation).take() else { return };
    let samples = rec.stop();
    resume_meeting_mic(app);
    status(app, "transcribing");
    let app = app.clone();
    thread::spawn(move || {
        if let Err(e) = transcribe_and_paste(&app, samples) {
            log::warn!("dictation failed: {e:#}");
        }
        status(&app, "idle");
    });
}

pub fn toggle(app: &Arc<App>) {
    if is_recording(app) {
        stop(app)
    } else {
        start(app)
    }
}

pub fn cancel(app: &App) {
    if lock(&app.dictation).take().is_some() {
        resume_meeting_mic(app);
        status(app, "idle");
    }
}

fn resume_meeting_mic(app: &App) {
    if let Some(m) = lock(&app.recording).as_ref() {
        m.recorder.pause_mic(false);
    }
}

fn transcribe_and_paste(app: &App, samples: Vec<f32>) -> anyhow::Result<()> {
    let duration_ms = samples.len() as u64 * 1000 / SAMPLE_RATE as u64;
    let speech = match app.vad() {
        Some(vad) => vad.trim_silence(&samples),
        None => samples,
    };
    if speech.is_empty() {
        return Ok(());
    }
    let settings = app.settings();
    let transcriber = match app.whisper(&settings.dictation.model) {
        Ok(t) => t,
        Err(e) => {
            alert(app, Alert::ModelMissing);
            return Err(e);
        }
    };
    let language = (settings.transcription_language != "auto").then_some(settings.transcription_language.as_str());
    let transcript = {
        let _turn = app.gate.acquire(true);
        transcriber.transcribe(&speech, language)?
    };
    let text = transcript.segments.iter().map(|s| s.text.trim()).filter(|t| !t.is_empty()).collect::<Vec<_>>().join(" ");
    if text.is_empty() {
        return Ok(());
    }
    let injected_into = inject::active_app();
    let delays = inject::Delays {
        before_paste: Duration::from_millis(settings.dictation.paste_delay_before_ms),
        before_restore: Duration::from_millis(settings.dictation.paste_delay_after_ms),
    };
    // A failed paste is not fatal: the text stays in the history (spec 7.1).
    if let Err(e) = inject::paste_text(&text, &delays) {
        log::warn!("paste failed: {e:#}");
    }
    let dictation = Dictation {
        id: uuid::Uuid::new_v4(),
        created_at: chrono::Utc::now(),
        text,
        language: Some(transcript.language),
        duration_ms,
        injected_into,
        provider: transcriber.name().to_string(),
    };
    if settings.dictation.keep_history {
        app.db().add_dictation(&dictation)?;
    }
    let _ = app.handle.emit("dictation-done", &dictation);
    Ok(())
}

enum Alert {
    NoMic,
    MicDenied,
    ModelMissing,
}

/// Dictation runs with the window hidden: failures surface as system notifications.
fn alert(app: &App, alert: Alert) {
    let en = app.settings().ui_language == "en";
    let body = match (alert, en) {
        (Alert::NoMic, false) => "Aucun micro détecté.",
        (Alert::NoMic, true) => "No microphone found.",
        (Alert::MicDenied, false) => "Accès au micro refusé. Autorisez Barnaby dans les réglages du système.",
        (Alert::MicDenied, true) => "Microphone access denied. Allow Barnaby in the system settings.",
        (Alert::ModelMissing, false) => "Modèle de dictée absent. Téléchargez-le dans Réglages › Modèles.",
        (Alert::ModelMissing, true) => "Dictation model missing. Download it in Settings › Models.",
    };
    let _ = app.handle.notification().builder().title("Barnaby").body(body).show();
}
