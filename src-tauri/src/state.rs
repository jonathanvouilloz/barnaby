//! Process-wide state shared by commands, shortcuts and worker threads.

use std::{
    collections::HashMap,
    path::PathBuf,
    sync::{mpsc::Sender, Arc, Mutex, MutexGuard},
    time::Instant,
};

use barnaby_core::{
    store::{SharedStore, Store},
    Settings, Transcriber,
};
use barnaby_engine::{models, ParakeetTranscriber, Recorder, SherpaDiarizer, WhisperTranscriber};
use tauri::{AppHandle, Emitter};
use uuid::Uuid;

use crate::{dictation::Session, sched::Gate};

pub enum Job {
    Process(Uuid),
    Send(Uuid),
}

pub struct ActiveMeeting {
    pub id: Uuid,
    pub recorder: Recorder,
    pub started: Instant,
}

pub struct App {
    pub handle: AppHandle,
    pub store: SharedStore,
    pub data_dir: PathBuf,
    pub models_dir: PathBuf,
    pub gate: Arc<Gate>,
    pub jobs: Sender<Job>,
    pub recording: Mutex<Option<ActiveMeeting>>,
    pub dictation: Mutex<Option<Session>>,
    transcribers: Mutex<HashMap<String, Arc<dyn Transcriber>>>,
    diarizer: Mutex<Option<Arc<SherpaDiarizer>>>,
}

pub const KEYRING_SERVICE: &str = "barnaby";
pub const WEBHOOK_SECRET_USER: &str = "webhook-secret";
pub const LLM_API_KEY_USER: &str = "llm-api-key";

/// Error codes cross the IPC boundary as plain strings (`error.<code>` in the UI).
pub type CmdResult<T> = Result<T, String>;

pub fn code(code: &'static str) -> impl Fn(anyhow::Error) -> String {
    move |e| {
        log::warn!("{code}: {e:#}");
        code.to_string()
    }
}

impl App {
    pub fn new(handle: AppHandle, store: Store, data_dir: PathBuf, jobs: Sender<Job>) -> Self {
        Self {
            handle,
            store: Arc::new(Mutex::new(store)),
            models_dir: data_dir.join("models"),
            data_dir,
            gate: Arc::default(),
            jobs,
            recording: Mutex::default(),
            dictation: Mutex::default(),
            transcribers: Mutex::default(),
            diarizer: Mutex::default(),
        }
    }

    pub fn db(&self) -> MutexGuard<'_, Store> {
        barnaby_core::store::lock(&self.store)
    }

    pub fn settings(&self) -> Settings {
        self.db().settings().unwrap_or_default()
    }

    pub fn emit_meeting(&self, id: Uuid) {
        if let Ok(m) = self.db().get_meeting(id) {
            let _ = self.handle.emit("meeting-updated", m);
        }
        crate::tray::refresh(self);
    }

    pub fn webhook_secret(&self) -> Option<String> {
        secret(WEBHOOK_SECRET_USER)
    }

    pub fn llm_api_key(&self) -> Option<String> {
        secret(LLM_API_KEY_USER)
    }

    /// Loaded once per model id; loading a large model takes seconds.
    pub fn transcriber(&self, model_id: &str) -> anyhow::Result<Arc<dyn Transcriber>> {
        let mut cache = lock(&self.transcribers);
        if let Some(t) = cache.get(model_id) {
            return Ok(t.clone());
        }
        let path = models::path_for(&self.models_dir, model_id).ok_or_else(|| anyhow::anyhow!("model_missing"))?;
        let parakeet = models::catalog().iter().any(|m| m.id == model_id && m.kind == models::ModelKind::Parakeet);
        let t: Arc<dyn Transcriber> = match path.parent() {
            // The catalog path of an archive model is a file inside its directory.
            Some(dir) if parakeet => Arc::new(ParakeetTranscriber::load(dir)?),
            _ => Arc::new(WhisperTranscriber::load(&path, true)?),
        };
        cache.insert(model_id.to_string(), t.clone());
        Ok(t)
    }

    /// Loads the dictation model in the background, so the first dictation doesn't wait for it.
    pub fn preload_dictation_model(self: &Arc<Self>) {
        let app = self.clone();
        std::thread::spawn(move || {
            let id = app.settings().dictation.model;
            if let Err(e) = app.transcriber(&id) {
                log::info!("dictation model {id} not preloaded: {e:#}");
            }
        });
    }

    pub fn diarizer(&self) -> anyhow::Result<Arc<SherpaDiarizer>> {
        let mut cache = lock(&self.diarizer);
        if let Some(d) = cache.as_ref() {
            return Ok(d.clone());
        }
        let find = |kind| {
            models::catalog()
                .iter()
                .filter(|m| m.kind == kind)
                .find_map(|m| models::path_for(&self.models_dir, m.id))
                .ok_or_else(|| anyhow::anyhow!("model_missing"))
        };
        let d = Arc::new(SherpaDiarizer::load(
            &find(models::ModelKind::DiarizationSegmentation)?,
            &find(models::ModelKind::DiarizationEmbedding)?,
        )?);
        *cache = Some(d.clone());
        Ok(d)
    }

    pub fn forget_models(&self) {
        lock(&self.transcribers).clear();
        *lock(&self.diarizer) = None;
    }
}

fn secret(user: &str) -> Option<String> {
    keyring::Entry::new(KEYRING_SERVICE, user).ok()?.get_password().ok()
}

pub fn lock<T>(m: &Mutex<T>) -> MutexGuard<'_, T> {
    m.lock().unwrap_or_else(|e| e.into_inner())
}
