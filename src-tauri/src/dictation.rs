//! Dictation (spec 7): mic → streaming VAD → each speech segment transcribed while the
//! user talks (ahead of meetings) → on stop, only the tail is left → paste at cursor.

use std::{
    sync::{
        mpsc::{self, Receiver, RecvTimeoutError, Sender},
        Arc,
    },
    thread,
    time::Duration,
};

use anyhow::anyhow;
use barnaby_core::{audio::rms, Dictation, Settings, Transcript, SAMPLE_RATE};
use barnaby_engine::{cue, models, DictationRecorder, LiveDictation, VadStream};
use tauri::Emitter;
use tauri_plugin_notification::NotificationExt;

use crate::{
    inject, overlay,
    state::{lock, App},
};

/// Mic polling period: also paces `dictation-level` (20 per second).
const TICK: Duration = Duration::from_millis(50);

/// The running dictation, owned by its worker thread; this is how it is told to end.
pub struct Session {
    end: Sender<End>,
}

/// The running onboarding mic test.
pub struct MicTest {
    end: Sender<()>,
}

enum End {
    Stop,
    Cancel,
}

/// Emits `mic-level` until `stop_mic_test`; the samples are only measured, never kept.
pub fn start_mic_test(app: &Arc<App>) -> Result<(), String> {
    let mut slot = lock(&app.mic_test);
    if slot.is_some() {
        return Ok(());
    }
    let mut recorder = DictationRecorder::start().map_err(|e| {
        log::warn!("mic test start: {e:#}");
        e.to_string()
    })?;
    let (end, rx) = mpsc::channel();
    *slot = Some(MicTest { end });
    let handle = app.handle.clone();
    thread::spawn(move || {
        while let Err(RecvTimeoutError::Timeout) = rx.recv_timeout(TICK) {
            let _ = handle.emit("mic-level", level(&recorder.drain()));
        }
        recorder.stop();
    });
    Ok(())
}

pub fn stop_mic_test(app: &App) {
    if let Some(t) = lock(&app.mic_test).take() {
        let _ = t.end.send(());
    }
}

fn status(app: &App, s: &str) {
    let _ = app.handle.emit("dictation-status", s);
    match s {
        "recording" if app.settings().dictation.show_overlay => overlay::show(&app.handle),
        "idle" => overlay::hide(&app.handle),
        _ => {}
    }
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
    let recorder = match DictationRecorder::start() {
        Ok(r) => r,
        Err(e) => {
            log::warn!("dictation start: {e:#}");
            alert(app, if e.to_string() == "capture.permission_denied" { Alert::MicDenied } else { Alert::NoMic });
            return;
        }
    };
    let (end, rx) = mpsc::channel();
    *slot = Some(Session { end });
    drop(slot);
    // Spec 7.4: the mic is shared; keep the dictation out of a meeting's mic track.
    if let Some(m) = lock(&app.recording).as_ref() {
        m.recorder.pause_mic(true);
    }
    status(app, "recording");
    // The mic hears the start cue: its first samples are zeroed so it is never transcribed.
    let mute = if app.settings().dictation.sound_cues {
        thread::spawn(cue::play_start);
        cue::MIC_MUTE.as_millis() as usize * SAMPLE_RATE as usize / 1000
    } else {
        0
    };
    let app = app.clone();
    thread::spawn(move || run(&app, recorder, rx, mute));
}

pub fn stop(app: &Arc<App>) {
    end(app, End::Stop, "transcribing");
}

pub fn toggle(app: &Arc<App>) {
    if is_recording(app) {
        stop(app)
    } else {
        start(app)
    }
}

pub fn cancel(app: &App) {
    end(app, End::Cancel, "idle");
}

fn end(app: &App, how: End, next: &str) {
    let Some(session) = lock(&app.dictation).take() else { return };
    let _ = session.end.send(how);
    resume_meeting_mic(app);
    status(app, next);
}

fn resume_meeting_mic(app: &App) {
    if let Some(m) = lock(&app.recording).as_ref() {
        m.recorder.pause_mic(false);
    }
}

/// Worker thread of one dictation, from the first sample to the paste.
fn run(app: &Arc<App>, mut recorder: DictationRecorder, end: Receiver<End>, mute: usize) {
    let settings = app.settings();
    // Without the VAD model the whole recording is transcribed at the end.
    let vad = models::path_for(&app.models_dir, "silero-vad").and_then(|p| VadStream::new(&p).inspect_err(|e| log::warn!("vad: {e:#}")).ok());
    let mut live = LiveDictation::new(vad, segment_transcriber(app.clone(), &settings));
    let mut samples = 0;
    let stopped = loop {
        match end.recv_timeout(TICK) {
            Err(RecvTimeoutError::Timeout) => {
                let mut chunk = recorder.drain();
                let quiet = mute.saturating_sub(samples).min(chunk.len());
                chunk[..quiet].fill(0.0);
                let _ = app.handle.emit("dictation-level", level(&chunk));
                samples += chunk.len();
                live.push(&chunk);
            }
            Ok(End::Stop) | Err(RecvTimeoutError::Disconnected) => break true,
            Ok(End::Cancel) => break false,
        }
    };
    let tail = recorder.stop();
    let _ = app.handle.emit("dictation-level", 0.0f32);
    if !stopped {
        return;
    }
    samples += tail.len();
    live.push(&tail);
    let duration_ms = samples as u64 * 1000 / u64::from(SAMPLE_RATE);
    if let Err(e) = live.finish().and_then(|t| deliver(app, &settings, &t, duration_ms)) {
        log::warn!("dictation failed: {e:#}");
        if e.to_string() == "model_missing" {
            alert(app, Alert::ModelMissing);
        }
    }
    // A new dictation may have started while this one was transcribing.
    if !is_recording(app) {
        status(app, "idle");
    }
}

/// One speech segment at a time, ahead of any meeting (gate) and on the cached model.
fn segment_transcriber(app: Arc<App>, settings: &Settings) -> impl FnMut(&[f32]) -> anyhow::Result<Transcript> + Send + 'static {
    let model = settings.dictation.model.clone();
    let language = (settings.transcription_language != "auto").then(|| settings.transcription_language.clone());
    move |speech| {
        let transcriber = app.transcriber(&model).map_err(|e| {
            log::warn!("dictation model {model}: {e:#}");
            anyhow!("model_missing")
        })?;
        let _turn = app.gate.acquire(true);
        transcriber.transcribe(speech, language.as_deref())
    }
}

/// Mic RMS mapped from -60..-10 dBFS to 0..1, so normal speech sits mid-scale.
pub(crate) fn level(chunk: &[f32]) -> f32 {
    let db = 20.0 * rms(chunk).max(1e-6).log10();
    ((db + 60.0) / 50.0).clamp(0.0, 1.0)
}

fn deliver(app: &App, settings: &Settings, transcript: &Transcript, duration_ms: u64) -> anyhow::Result<()> {
    let text = transcript.segments.iter().map(|s| s.text.trim()).filter(|t| !t.is_empty()).collect::<Vec<_>>().join(" ");
    let text = barnaby_core::vocabulary::correct(&text, &settings.vocabulary);
    if text.is_empty() {
        return Ok(());
    }
    let injected_into = inject::active_app();
    // Nothing to paste into (desktop, taskbar): the text waits on the clipboard, say so.
    let no_target = inject::no_target() == Some(true);
    // A failed paste is not fatal: the text stays in the history and on the clipboard (spec 7.1).
    if let Err(e) = inject::paste_text(&text, Duration::from_millis(settings.dictation.paste_delay_before_ms), !no_target) {
        log::warn!("paste failed: {e:#}");
    }
    if no_target {
        alert(app, Alert::Copied);
    }
    let dictation = Dictation {
        id: uuid::Uuid::new_v4(),
        created_at: chrono::Utc::now(),
        text,
        language: Some(transcript.language.clone()).filter(|l| !l.is_empty() && l != "auto"),
        duration_ms,
        injected_into,
        provider: app.transcriber(&settings.dictation.model).map(|t| t.name().to_string()).unwrap_or_default(),
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
    Copied,
}

/// Dictation runs with the window hidden: failures surface as system notifications.
fn alert(app: &App, alert: Alert) {
    let en = app.settings().ui_language == "en";
    let body = match (alert, en) {
        (Alert::NoMic, false) => "Aucun micro détecté.",
        (Alert::NoMic, true) => "No microphone found.",
        (Alert::MicDenied, false) => "Accès au micro refusé. Autorise Barnaby dans les réglages du système.",
        (Alert::MicDenied, true) => "Microphone access denied. Allow Barnaby in the system settings.",
        (Alert::ModelMissing, false) => "Modèle de dictée absent. Télécharge-le dans Réglages › Modèles.",
        (Alert::ModelMissing, true) => "Dictation model missing. Download it in Settings › Models.",
        (Alert::Copied, false) => "Texte copié, tu n’as plus qu’à coller.",
        (Alert::Copied, true) => "Text copied, just paste it.",
    };
    let _ = app.handle.notification().builder().title("Barnaby").body(body).show();
}

#[cfg(test)]
mod tests {
    use super::level;

    #[test]
    fn level_scale() {
        assert_eq!(level(&[]), 0.0);
        assert_eq!(level(&[1.0; 100]), 1.0);
        let speech = level(&[0.03; 800]); // about -30 dBFS
        assert!((0.5..0.7).contains(&speech), "{speech}");
    }
}
