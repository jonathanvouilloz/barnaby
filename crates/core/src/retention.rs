//! Meeting files on disk and audio retention (spec section 16).

use std::path::{Path, PathBuf};

use anyhow::Result;
use chrono::{DateTime, Duration, Utc};
use uuid::Uuid;

use crate::model::MeetingState;
use crate::store::Store;

pub const MIC_WAV: &str = "mic.wav";
pub const SYSTEM_WAV: &str = "system.wav";

/// `<data_dir>/meetings/<id>/`
pub fn meeting_dir(data_dir: &Path, id: Uuid) -> PathBuf {
    data_dir.join("meetings").join(id.to_string())
}

/// Removes the meeting's WAV files; the transcript is never deleted automatically.
pub fn delete_audio(data_dir: &Path, id: Uuid) -> Result<()> {
    for name in [MIC_WAV, SYSTEM_WAV] {
        match std::fs::remove_file(meeting_dir(data_dir, id).join(name)) {
            Err(e) if e.kind() != std::io::ErrorKind::NotFound => return Err(e.into()),
            _ => {}
        }
    }
    Ok(())
}

/// `SevenDays` retention: drops audio of reviewed or sent meetings ended more than
/// 7 days before `now`. Meetings still to process keep their audio.
pub fn purge_expired_audio(store: &Store, data_dir: &Path, now: DateTime<Utc>) -> Result<()> {
    for m in store.list_meetings_in(&[MeetingState::Reviewed, MeetingState::Sent])? {
        if m.ended_at.is_some_and(|t| now - t > Duration::days(7)) {
            delete_audio(data_dir, m.id)?;
        }
    }
    Ok(())
}

/// Deletes a meeting entirely: rows and its whole folder.
pub fn delete_meeting(store: &Store, data_dir: &Path, id: Uuid) -> Result<()> {
    store.delete_meeting(id)?;
    match std::fs::remove_dir_all(meeting_dir(data_dir, id)) {
        Err(e) if e.kind() != std::io::ErrorKind::NotFound => Err(e.into()),
        _ => Ok(()),
    }
}
