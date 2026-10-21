//! Sending a reviewed meeting, then applying audio retention.

use std::path::Path;
use std::sync::Mutex;

use anyhow::Result;
use chrono::Utc;
use uuid::Uuid;

use crate::model::{AudioRetention, MeetingState};
use crate::provider::Destination;
use crate::retention::{delete_audio, purge_expired_audio};
use crate::store::{lock, Store};

/// Sending → Sent, or Failed("send_failed:<destination>") with all data kept.
/// The store lock is released while destinations run (webhook retries can be slow).
pub fn send_meeting(
    store: &Mutex<Store>,
    destinations: &[Box<dyn Destination>],
    data_dir: &Path,
    id: Uuid,
) -> Result<()> {
    let (doc, retention) = {
        let db = lock(store);
        db.set_state(id, MeetingState::Sending, None)?;
        (db.meeting_doc(id)?, db.settings()?.meeting.retention)
    };
    for d in destinations {
        if let Err(e) = d.send(&doc) {
            log::warn!("meeting {id} send to {} failed", d.name());
            lock(store).set_state(
                id,
                MeetingState::Failed,
                Some(&format!("send_failed:{}", d.name())),
            )?;
            return Err(e);
        }
    }
    let db = lock(store);
    db.set_state(id, MeetingState::Sent, None)?;
    match retention {
        AudioRetention::OnReview => delete_audio(data_dir, id),
        AudioRetention::SevenDays => purge_expired_audio(&db, data_dir, Utc::now()),
        AudioRetention::Forever => Ok(()),
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::destination::{tests::serve, FolderDestination, WebhookDestination};
    use crate::model::{ProcessingMode, Settings};
    use crate::retention::{meeting_dir, MIC_WAV};
    use std::time::Duration;

    fn setup() -> (Mutex<Store>, std::path::PathBuf, Uuid) {
        let data_dir = std::env::temp_dir().join(format!("barnaby-send-{}", Uuid::new_v4()));
        let store = Store::open_in_memory().unwrap();
        let id = store
            .create_meeting("t", &[], ProcessingMode::Local)
            .unwrap()
            .id;
        store.set_state(id, MeetingState::Reviewed, None).unwrap();
        std::fs::create_dir_all(meeting_dir(&data_dir, id)).unwrap();
        std::fs::write(meeting_dir(&data_dir, id).join(MIC_WAV), b"x").unwrap();
        (Mutex::new(store), data_dir, id)
    }

    #[test]
    fn failed_send_keeps_everything_then_success_applies_retention() {
        let (store, dir, id) = setup();
        let out = dir.join("out");
        let (url, _rx) = serve(vec![(400, String::new())]);
        let dests: Vec<Box<dyn Destination>> = vec![
            Box::new(FolderDestination { path: out.clone() }),
            Box::new(WebhookDestination {
                url,
                secret: None,
                retry_base: Duration::from_millis(1),
            }),
        ];
        assert!(send_meeting(&store, &dests, &dir, id).is_err());
        let m = lock(&store).get_meeting(id).unwrap();
        assert_eq!(
            (m.state, m.error.as_deref()),
            (MeetingState::Failed, Some("send_failed:webhook"))
        );
        assert!(meeting_dir(&dir, id).join(MIC_WAV).exists());

        send_meeting(&store, &dests[..1], &dir, id).unwrap();
        assert_eq!(
            lock(&store).get_meeting(id).unwrap().state,
            MeetingState::Sent
        );
        assert!(out.join(format!("{id}.json")).exists());
        assert!(!meeting_dir(&dir, id).join(MIC_WAV).exists());
        std::fs::remove_dir_all(dir).unwrap();
    }

    #[test]
    fn seven_days_and_forever_retention() {
        let (store, dir, id) = setup();
        let mut settings = Settings::default();
        settings.meeting.retention = crate::model::AudioRetention::SevenDays;
        lock(&store).save_settings(&settings).unwrap();
        send_meeting(&store, &[], &dir, id).unwrap();
        let wav = meeting_dir(&dir, id).join(MIC_WAV);
        assert!(wav.exists(), "ended just now: kept");
        purge_expired_audio(&lock(&store), &dir, Utc::now() + chrono::Duration::days(8)).unwrap();
        assert!(!wav.exists());

        let (store, dir2, id) = setup();
        settings.meeting.retention = crate::model::AudioRetention::Forever;
        lock(&store).save_settings(&settings).unwrap();
        send_meeting(&store, &[], &dir2, id).unwrap();
        assert!(meeting_dir(&dir2, id).join(MIC_WAV).exists());
        for d in [dir, dir2] {
            std::fs::remove_dir_all(d).unwrap();
        }
    }
}
