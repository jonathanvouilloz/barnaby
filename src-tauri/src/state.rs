//! Process-wide state shared by commands, shortcuts and worker threads.

use std::{
    collections::HashMap,
    path::PathBuf,
    sync::{mpsc::Sender, Arc, Mutex, MutexGuard},
    time::Instant,
};

use barnaby_core::{
    store::{SharedStore, Store},
    Settings,
};
use barnaby_engine::{models, DictationRecorder, Recorder, SherpaDiarizer, Vad, WhisperTranscriber};
use tauri::{AppHandle, Emitter};
use uuid::Uuid;

use crate::sched::Gate;

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
    pub dictation: Mutex<Option<DictationRecorder>>,
    whisper: Mutex<HashMap<String, Arc<WhisperTranscriber>>>,
    diarizer: Mutex<Option<Arc<SherpaDiarizer>>>,
    vad: Mutex<Option<Arc<Vad>>>,
}

pub const KEYRING_SERVICE: &str = "barnaby";
pub const WEBHOOK_SECRET_USER: &str = "webhook-secret";

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
            whisper: Mutex::default(),
            diarizer: Mutex::default(),
            vad: Mutex::default(),
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
        keyring::Entry::new(KEYRING_SERVICE, WEBHOOK_SECRET_USER).ok()?.get_password().ok()
    }

    /// Loaded once per model id; loading a large model takes seconds.
    pub fn whisper(&self, model_id: &str) -> anyhow::Result<Arc<WhisperTranscriber>> {
        let mut cache = lock(&self.whisper);
        if let Some(t) = cache.get(model_id) {
            return Ok(t.clone());
        }
        let path = models::path_for(&self.models_dir, model_id).ok_or_else(|| anyhow::anyhow!("model_missing"))?;
        let t = Arc::new(WhisperTranscriber::load(&path, true)?);
        cache.insert(model_id.to_string(), t.clone());
        Ok(t)
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

    /// None when the VAD model isn't installed: dictation then keeps the raw audio.
    pub fn vad(&self) -> Option<Arc<Vad>> {
        let mut cache = lock(&self.vad);
        if cache.is_none() {
            let path = models::path_for(&self.models_dir, "silero-vad")?;
            *cache = Vad::load(&path).ok().map(Arc::new);
        }
        cache.clone()
    }

    pub fn forget_models(&self) {
        lock(&self.whisper).clear();
        *lock(&self.diarizer) = None;
        *lock(&self.vad) = None;
    }
}

pub fn lock<T>(m: &Mutex<T>) -> MutexGuard<'_, T> {
    m.lock().unwrap_or_else(|e| e.into_inner())
}
