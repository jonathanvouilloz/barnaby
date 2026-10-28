//! Meeting lifecycle: recording, then a single worker thread that processes and sends.

use std::{
    sync::{mpsc::Receiver, Arc},
    thread,
    time::{Duration, Instant},
};

use anyhow::{bail, Result};
use barnaby_core::{
    destination, llm::OpenAiStructurer, mistral::{MistralTranscriber, MISTRAL_MODEL}, net::NetPolicy,
    pipeline::Pipeline, retention, send::send_meeting,
    AudioRetention, MeetingState, Participant, ProcessingMode, Settings, Structurer, Transcriber,
};
use barnaby_engine::Recorder;
use serde_json::json;
use tauri::Emitter;
use tauri_plugin_notification::NotificationExt;
use uuid::Uuid;

use crate::{
    sched::MeetingTranscriber,
    state::{lock, ActiveMeeting, App, Job},
};

pub fn start(app: &Arc<App>, title: &str, participants: &[Participant]) -> Result<Uuid> {
    let mut recording = lock(&app.recording);
    if recording.is_some() {
        bail!("already_recording");
    }
    let mode = app.settings().processing_mode;
    let meeting = app.db().create_meeting(title, participants, mode)?;
    let dir = retention::meeting_dir(&app.data_dir, meeting.id);
    let recorder = std::fs::create_dir_all(&dir).map_err(anyhow::Error::from).and_then(|_| Recorder::start(&dir, &app.settings().meeting.excluded_outputs));
    let recorder = match recorder {
        Ok(r) => r,
        Err(e) => {
            // Nothing was recorded: don't leave an empty meeting behind.
            let _ = retention::delete_meeting(&app.db(), &app.data_dir, meeting.id);
            return Err(e);
        }
    };
    // Spec 7.4: a dictation already running keeps the mic out of the meeting track.
    if lock(&app.dictation).is_some() {
        recorder.pause_mic(true);
    }
    *recording = Some(ActiveMeeting { id: meeting.id, recorder, started: Instant::now() });
    drop(recording);
    app.emit_meeting(meeting.id);
    spawn_level_monitor(app.clone(), meeting.id);
    log::info!("meeting {} recording", meeting.id);
    Ok(meeting.id)
}

pub fn stop(app: &App) -> Result<()> {
    let Some(active) = lock(&app.recording).take() else { bail!("not_recording") };
    let stopped = active.recorder.stop();
    // Even if finalizing failed, the files are on disk and the pipeline repairs their headers.
    if let Err(e) = &stopped {
        log::warn!("meeting {} stop: {e:#}", active.id);
    }
    app.db().set_state(active.id, MeetingState::Queued, None)?;
    app.emit_meeting(active.id);
    let _ = app.jobs.send(Job::Process(active.id));
    Ok(())
}

/// Meeting shortcut: stop if recording, else start with a dated default title.
pub fn toggle(app: &Arc<App>) -> Result<()> {
    if lock(&app.recording).is_some() {
        return stop(app);
    }
    if !app.settings().consent_acknowledged {
        // The consent reminder (spec 16) lives in the new-meeting screen.
        crate::show_main(&app.handle, Some("new"));
        return Ok(());
    }
    let title = chrono::Local::now().format("Réunion du %d.%m.%Y %H:%M").to_string();
    start(app, &title, &[]).map(drop)
}

/// Timestamped note on the meeting being recorded ("manual" from the UI, "marker" from the shortcut).
pub fn add_note(app: &App, text: &str, source: &str) -> Result<()> {
    let (id, at) = {
        let rec = lock(&app.recording);
        let Some(a) = rec.as_ref() else { bail!("not_recording") };
        (a.id, a.started.elapsed().as_secs_f64())
    };
    app.db().add_note(id, at, text, source)?;
    Ok(())
}

fn spawn_level_monitor(app: Arc<App>, id: Uuid) {
    thread::spawn(move || {
        let mut shown: Vec<&'static str> = Vec::new();
        loop {
            let (levels, warnings) = {
                let rec = lock(&app.recording);
                match rec.as_ref() {
                    Some(a) if a.id == id => (a.recorder.levels(), a.recorder.warnings()),
                    _ => return,
                }
            };
            let _ = app.handle.emit("audio-level", json!({ "mic": levels.0, "system": levels.1 }));
            for w in warnings {
                if !shown.contains(&w) {
                    shown.push(w);
                    let _ = app.handle.emit("capture-warning", w);
                }
            }
            thread::sleep(Duration::from_millis(100));
        }
    });
}

/// After a crash: a meeting left in `recording` has its audio on disk; queue it.
pub fn recover(app: &App) -> Result<()> {
    let db = app.db();
    for m in db.list_meetings_in(&[MeetingState::Recording])? {
        db.set_state(m.id, MeetingState::Captured, None)?;
    }
    for m in db.list_meetings_in(&[MeetingState::Captured, MeetingState::Queued, MeetingState::Processing])? {
        let _ = app.jobs.send(Job::Process(m.id));
    }
    for m in db.list_meetings_in(&[MeetingState::Reviewed, MeetingState::Sending])? {
        let _ = app.jobs.send(Job::Send(m.id));
    }
    if db.settings()?.meeting.retention == AudioRetention::SevenDays {
        retention::purge_expired_audio(&db, &app.data_dir, chrono::Utc::now())?;
    }
    Ok(())
}

/// One meeting at a time: processing is CPU/GPU bound and shares the transcription gate.
pub fn worker(app: Arc<App>, jobs: Receiver<Job>) {
    for job in jobs {
        match job {
            Job::Process(id) => process(&app, id),
            Job::Send(id) => send(&app, id),
        }
    }
}

fn process(app: &App, id: Uuid) {
    let settings = app.settings();
    // The meeting's own mode, not the current setting: a local meeting stays local when retried.
    let Ok(mode) = app.db().get_meeting(id).map(|m| m.mode) else { return };
    let fail = |code: &str| {
        let _ = app.db().set_state(id, MeetingState::Failed, Some(code));
        app.emit_meeting(id);
    };
    let transcriber: Arc<dyn Transcriber> = match uses_mistral(&settings.meeting.model, mode) {
        Err(code) => return fail(code),
        Ok(true) => {
            let Some(api_key) = app.mistral_api_key() else { return fail("mistral_key_missing") };
            Arc::new(MistralTranscriber { api_key, vocabulary: settings.vocabulary.clone(), policy: NetPolicy { mode } })
        }
        Ok(false) => {
            let Ok(inner) = app.transcriber(&settings.meeting.model) else { return fail("model_missing") };
            Arc::new(MeetingTranscriber { inner, gate: app.gate.clone() })
        }
    };
    let Ok(diarizer) = app.diarizer() else { return fail("diarization_model_missing") };
    let structurer = settings.llm.enabled.then(|| {
        Arc::new(OpenAiStructurer {
            base_url: settings.llm.base_url.clone(),
            model: settings.llm.model.clone(),
            api_key: app.llm_api_key(),
            vocabulary: settings.vocabulary.clone(),
            policy: NetPolicy { mode },
        }) as Arc<dyn Structurer>
    });
    let pipeline = Pipeline {
        store: app.store.clone(),
        transcriber,
        diarizer,
        structurer,
        data_dir: app.data_dir.clone(),
        progress: None,
    };
    // Pipeline::run sets `processing` too, but only returns at the end: tell the UI now.
    let _ = app.db().set_state(id, MeetingState::Processing, None);
    app.emit_meeting(id);
    let ok = pipeline.run(id).is_ok();
    app.emit_meeting(id);
    if let Ok(m) = app.db().get_meeting(id) {
        notify(app, &m.title, ok, &settings);
    }
}

/// Mistral only for a meeting recorded in `api` mode: a local or hybrid one was promised its audio stays here.
fn uses_mistral(model: &str, mode: ProcessingMode) -> Result<bool, &'static str> {
    match (model == MISTRAL_MODEL, mode) {
        (false, _) => Ok(false),
        (true, ProcessingMode::Api) => Ok(true),
        (true, _) => Err("mistral_mode_mismatch"),
    }
}

fn send(app: &App, id: Uuid) {
    let settings = app.settings();
    let destinations = destination::from_settings(&settings.destinations, app.webhook_secret());
    let _ = send_meeting(&app.store, &destinations, &app.data_dir, id);
    app.emit_meeting(id);
}

/// System notification at the end of processing (spec 13): silent unless the user enabled sound.
fn notify(app: &App, title: &str, ok: bool, settings: &Settings) {
    let body = match (ok, settings.ui_language == "en") {
        (true, false) => "Transcription prête à relire.",
        (true, true) => "Transcript ready for review.",
        (false, false) => "Le traitement a échoué. Ouvre Barnaby pour réessayer.",
        (false, true) => "Processing failed. Open Barnaby to retry.",
    };
    let mut n = app.handle.notification().builder().title(title).body(body);
    if settings.notification_sound {
        n = n.sound("default");
    }
    let _ = n.show();
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn only_api_meetings_reach_mistral() {
        assert_eq!(uses_mistral(MISTRAL_MODEL, ProcessingMode::Api), Ok(true));
        assert_eq!(uses_mistral(MISTRAL_MODEL, ProcessingMode::Local), Err("mistral_mode_mismatch"));
        assert_eq!(uses_mistral(MISTRAL_MODEL, ProcessingMode::Hybrid), Err("mistral_mode_mismatch"));
        assert_eq!(uses_mistral("whisper-small", ProcessingMode::Api), Ok(false));
    }
}
