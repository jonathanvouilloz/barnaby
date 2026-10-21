//! IPC commands. The contract is `ui/src/lib/api.ts`.

use std::{collections::BTreeSet, sync::{Arc, Mutex}};

use barnaby_core::{
    retention::{self, MIX_WAV},
    review, Dictation, Meeting, MeetingDoc, MeetingState, Participant, Resolution, ReviewItem, Settings,
};
use barnaby_engine::models;
use serde::Serialize;
use tauri::{Emitter, State};
use uuid::Uuid;

use crate::{
    meeting, shortcuts,
    state::{code, lock, App, CmdResult, Job, KEYRING_SERVICE, LLM_API_KEY_USER, WEBHOOK_SECRET_USER},
};

type AppState<'a> = State<'a, Arc<App>>;

fn parse_id(id: &str) -> CmdResult<Uuid> {
    Uuid::parse_str(id).map_err(|_| "invalid_id".to_string())
}

#[tauri::command]
pub fn get_settings(app: AppState) -> CmdResult<Settings> {
    app.db().settings().map_err(code("store_failed"))
}

#[tauri::command]
pub fn save_settings(app: AppState, settings: Settings) -> CmdResult<()> {
    let previous = app.settings();
    if let Err(e) = shortcuts::register(&app.handle, &settings) {
        // Keep the app usable with the shortcuts that worked before.
        let _ = shortcuts::register(&app.handle, &previous);
        return Err(e);
    }
    app.db().save_settings(&settings).map_err(code("store_failed"))?;
    if settings.dictation.model != previous.dictation.model {
        app.preload_dictation_model();
    }
    Ok(())
}

#[tauri::command]
pub fn set_webhook_secret(secret: String) -> CmdResult<()> {
    set_secret(WEBHOOK_SECRET_USER, &secret)
}

#[tauri::command]
pub fn has_webhook_secret(app: AppState) -> bool {
    app.webhook_secret().is_some()
}

#[tauri::command]
pub fn set_llm_api_key(key: String) -> CmdResult<()> {
    set_secret(LLM_API_KEY_USER, &key)
}

#[tauri::command]
pub fn has_llm_api_key(app: AppState) -> bool {
    app.llm_api_key().is_some()
}

/// An empty value deletes the secret.
fn set_secret(user: &str, secret: &str) -> CmdResult<()> {
    let entry = keyring::Entry::new(KEYRING_SERVICE, user).map_err(|_| "keyring_unavailable")?;
    let result = if secret.is_empty() {
        entry.delete_credential().or_else(|e| match e {
            keyring::Error::NoEntry => Ok(()),
            e => Err(e),
        })
    } else {
        entry.set_password(secret)
    };
    result.map_err(|_| "keyring_failed".to_string())
}

#[tauri::command]
pub fn list_meetings(app: AppState) -> CmdResult<Vec<Meeting>> {
    app.db().list_meetings().map_err(code("store_failed"))
}

#[derive(Serialize)]
pub struct MeetingDetail {
    meeting: Meeting,
    doc: MeetingDoc,
    review_items: Vec<ReviewItem>,
    audio_path: Option<String>,
}

#[tauri::command]
pub fn get_meeting(app: AppState, id: String) -> CmdResult<MeetingDetail> {
    let id = parse_id(&id)?;
    let db = app.db();
    let mix = retention::meeting_dir(&app.data_dir, id).join(MIX_WAV);
    Ok(MeetingDetail {
        meeting: db.get_meeting(id).map_err(code("store_failed"))?,
        doc: db.meeting_doc(id).map_err(code("store_failed"))?,
        review_items: db.review_items(id).map_err(code("store_failed"))?,
        audio_path: mix.exists().then(|| mix.to_string_lossy().into_owned()),
    })
}

#[tauri::command]
pub fn start_meeting(app: AppState, title: String, participants: Vec<Participant>) -> CmdResult<String> {
    meeting::start(&app, &title, &participants).map(|id| id.to_string()).map_err(|e| {
        let msg = e.to_string();
        // Capture errors from the engine are already `capture.*` codes the UI can show.
        if msg.starts_with("capture.") || msg == "already_recording" { msg } else { code("capture_failed")(e) }
    })
}

#[tauri::command]
pub fn stop_meeting(app: AppState) -> CmdResult<()> {
    meeting::stop(&app).map_err(code("stop_failed"))
}

#[tauri::command]
pub fn current_recording(app: AppState) -> Option<String> {
    lock(&app.recording).as_ref().map(|a| a.id.to_string())
}

#[tauri::command]
pub fn add_note(app: AppState, text: String) -> CmdResult<()> {
    meeting::add_note(&app, &text, "manual").map_err(code("note_failed"))
}

#[tauri::command]
pub fn resolve_review_item(app: AppState, id: String, resolution: Resolution) -> CmdResult<()> {
    let db = app.db();
    let meeting_id = db.get_review_item(&id).map_err(code("not_found"))?.meeting_id;
    review::apply_resolution(&db, &id, resolution).map_err(code("store_failed"))?;
    drop(db);
    app.emit_meeting(meeting_id);
    Ok(())
}

#[tauri::command]
pub fn rename_speaker(app: AppState, meeting_id: String, speaker_id: String, name: String) -> CmdResult<()> {
    let name = name.trim();
    if name.is_empty() {
        return Err("invalid_name".into());
    }
    app.db().rename_speaker(parse_id(&meeting_id)?, &speaker_id, name).map_err(code("store_failed"))
}

#[tauri::command]
pub fn update_segment_text(app: AppState, meeting_id: String, segment_id: String, text: String) -> CmdResult<()> {
    app.db().update_segment_text(parse_id(&meeting_id)?, &segment_id, &text).map_err(code("store_failed"))
}

#[tauri::command]
pub fn validate_review(app: AppState, id: String) -> CmdResult<()> {
    let id = parse_id(&id)?;
    let state = app.db().get_meeting(id).map_err(code("not_found"))?.state;
    if state != MeetingState::NeedsReview {
        return Err("invalid_state".into());
    }
    app.db().set_state(id, MeetingState::Reviewed, None).map_err(code("store_failed"))?;
    app.emit_meeting(id);
    // Spec 15: sending starts as soon as the review is validated.
    let _ = app.jobs.send(Job::Send(id));
    Ok(())
}

#[tauri::command]
pub fn retry_meeting(app: AppState, id: String) -> CmdResult<()> {
    let id = parse_id(&id)?;
    let m = app.db().get_meeting(id).map_err(code("not_found"))?;
    if m.state != MeetingState::Failed {
        return Err("invalid_state".into());
    }
    // A failed send keeps the reviewed transcript: only resend. Anything else resumes processing.
    let job = if m.error.as_deref().is_some_and(|e| e.starts_with("send_failed")) {
        app.db().set_state(id, MeetingState::Reviewed, None).map_err(code("store_failed"))?;
        Job::Send(id)
    } else {
        app.db().set_state(id, MeetingState::Queued, None).map_err(code("store_failed"))?;
        Job::Process(id)
    };
    app.emit_meeting(id);
    let _ = app.jobs.send(job);
    Ok(())
}

#[tauri::command]
pub fn delete_meeting(app: AppState, id: String) -> CmdResult<()> {
    let id = parse_id(&id)?;
    if lock(&app.recording).as_ref().is_some_and(|a| a.id == id) {
        return Err("still_recording".into());
    }
    retention::delete_meeting(&app.db(), &app.data_dir, id).map_err(code("store_failed"))
}

#[tauri::command]
pub fn list_dictations(app: AppState) -> CmdResult<Vec<Dictation>> {
    app.db().list_dictations().map_err(code("store_failed"))
}

#[tauri::command]
pub fn delete_dictation(app: AppState, id: String) -> CmdResult<()> {
    app.db().delete_dictation(parse_id(&id)?).map_err(code("store_failed"))
}

#[tauri::command]
pub fn clear_dictations(app: AppState) -> CmdResult<()> {
    app.db().clear_dictations().map_err(code("store_failed"))
}

#[derive(Serialize)]
pub struct ModelInfo {
    id: &'static str,
    kind: &'static str,
    name: &'static str,
    size_mb: u32,
    installed: bool,
}

#[tauri::command]
pub fn list_models(app: AppState) -> Vec<ModelInfo> {
    models::catalog()
        .iter()
        .map(|m| ModelInfo {
            id: m.id,
            kind: m.kind.as_str(),
            name: m.name,
            size_mb: m.size_mb,
            installed: models::installed(&app.models_dir, m.id),
        })
        .collect()
}

/// The one network action allowed in local mode, and only on the user's request (spec 6).
#[tauri::command]
pub async fn download_model(app: AppState<'_>, id: String) -> CmdResult<()> {
    // Two downloads of one model would write the same temporary files.
    static RUNNING: Mutex<BTreeSet<String>> = Mutex::new(BTreeSet::new());
    if !lock(&RUNNING).insert(id.clone()) {
        return Err("download_running".into());
    }
    let app = app.inner().clone();
    let running = id.clone();
    let result = tauri::async_runtime::spawn_blocking(move || {
        let handle = app.handle.clone();
        // One event per percent: the reader reports every chunk, thousands for a large model.
        let last = std::cell::Cell::new(-1);
        let progress = |p: f32| {
            let pct = (p * 100.0) as i32;
            if last.replace(pct) != pct {
                let _ = handle.emit("model-progress", serde_json::json!({ "id": id, "progress": p }));
            }
        };
        models::download(&app.models_dir, &id, progress).map(drop).map_err(code("download_failed"))
    })
    .await;
    lock(&RUNNING).remove(&running);
    result.map_err(|_| "download_failed".to_string())?
}

#[tauri::command]
pub fn delete_model(app: AppState, id: String) -> CmdResult<()> {
    if !models::installed(&app.models_dir, &id) {
        return Err("not_found".into());
    }
    app.forget_models();
    models::delete(&app.models_dir, &id).map_err(code("delete_failed"))
}
