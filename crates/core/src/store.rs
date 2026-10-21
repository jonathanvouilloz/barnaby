//! SQLite persistence. `Store` is `Send` but not `Sync`: the app shares it as
//! `SharedStore` (`Arc<Mutex<Store>>`) and never holds the lock during slow work.

use std::collections::BTreeMap;
use std::path::Path;
use std::sync::{Arc, Mutex, MutexGuard};

use anyhow::{Context, Result};
use chrono::{DateTime, Utc};
use rusqlite::{params, Connection, OptionalExtension, Row};
use serde::{de::DeserializeOwned, Serialize};
use uuid::Uuid;

use crate::model::*;

pub type SharedStore = Arc<Mutex<Store>>;

/// Locks the store, recovering from poisoning: SQLite keeps its own consistency.
pub fn lock(store: &Mutex<Store>) -> MutexGuard<'_, Store> {
    store.lock().unwrap_or_else(|e| e.into_inner())
}

const MIGRATIONS: &[&str] = &[r#"
CREATE TABLE meetings (
    id TEXT PRIMARY KEY,
    title TEXT NOT NULL,
    started_at TEXT NOT NULL,
    ended_at TEXT,
    language TEXT,
    state TEXT NOT NULL,
    error TEXT,
    mode TEXT NOT NULL,
    providers TEXT NOT NULL DEFAULT '{}'
);
CREATE TABLE participants (
    meeting_id TEXT NOT NULL REFERENCES meetings(id) ON DELETE CASCADE,
    pos INTEGER NOT NULL,
    id TEXT NOT NULL,
    name TEXT NOT NULL,
    email TEXT,
    confirmed INTEGER NOT NULL,
    PRIMARY KEY (meeting_id, id)
);
CREATE TABLE segments (
    meeting_id TEXT NOT NULL REFERENCES meetings(id) ON DELETE CASCADE,
    id TEXT NOT NULL,
    start REAL NOT NULL,
    end REAL NOT NULL,
    speaker_id TEXT,
    text TEXT NOT NULL,
    confidence REAL NOT NULL,
    reviewed INTEGER NOT NULL,
    PRIMARY KEY (meeting_id, id)
);
CREATE TABLE notes (
    meeting_id TEXT NOT NULL REFERENCES meetings(id) ON DELETE CASCADE,
    id TEXT NOT NULL,
    at REAL NOT NULL,
    text TEXT NOT NULL,
    source TEXT NOT NULL,
    PRIMARY KEY (meeting_id, id)
);
CREATE TABLE steps (
    meeting_id TEXT NOT NULL REFERENCES meetings(id) ON DELETE CASCADE,
    step TEXT NOT NULL,
    output TEXT NOT NULL,
    PRIMARY KEY (meeting_id, step)
);
CREATE TABLE review_items (
    id TEXT PRIMARY KEY,
    meeting_id TEXT NOT NULL REFERENCES meetings(id) ON DELETE CASCADE,
    segment_id TEXT,
    kind TEXT NOT NULL,
    start REAL NOT NULL,
    end REAL NOT NULL,
    proposed_text TEXT NOT NULL,
    alternatives TEXT NOT NULL,
    question TEXT NOT NULL,
    resolution TEXT
);
CREATE INDEX review_items_meeting ON review_items(meeting_id);
CREATE TABLE dictations (
    id TEXT PRIMARY KEY,
    created_at TEXT NOT NULL,
    text TEXT NOT NULL,
    language TEXT,
    duration_ms INTEGER NOT NULL,
    injected_into TEXT,
    provider TEXT NOT NULL
);
CREATE TABLE speaker_names (
    key TEXT PRIMARY KEY,
    name TEXT NOT NULL
);
CREATE TABLE settings (
    id INTEGER PRIMARY KEY CHECK (id = 1),
    json TEXT NOT NULL
);
"#];

/// Key under which the user's own display name is remembered (the "me" speaker).
const ME_KEY: &str = "@me";

pub struct Store {
    conn: Connection,
}

impl Store {
    pub fn open(path: &Path) -> Result<Self> {
        let conn = Connection::open(path)?;
        conn.pragma_update(None, "journal_mode", "WAL")?;
        Self::init(conn)
    }

    pub fn open_in_memory() -> Result<Self> {
        Self::init(Connection::open_in_memory()?)
    }

    fn init(mut conn: Connection) -> Result<Self> {
        conn.pragma_update(None, "foreign_keys", true)?;
        let version: i64 = conn.pragma_query_value(None, "user_version", |r| r.get(0))?;
        for (i, sql) in MIGRATIONS.iter().enumerate().skip(version as usize) {
            let tx = conn.transaction()?;
            tx.execute_batch(sql)?;
            tx.pragma_update(None, "user_version", i as i64 + 1)?;
            tx.commit()?;
        }
        Ok(Self { conn })
    }

    pub fn schema_version(&self) -> Result<usize> {
        let v: i64 = self
            .conn
            .pragma_query_value(None, "user_version", |r| r.get(0))?;
        Ok(v as usize)
    }

    // ---------- meetings ----------

    pub fn create_meeting(
        &self,
        title: &str,
        participants: &[Participant],
        mode: ProcessingMode,
    ) -> Result<Meeting> {
        let id = Uuid::new_v4();
        self.conn.execute(
            "INSERT INTO meetings (id, title, started_at, state, mode) VALUES (?1, ?2, ?3, ?4, ?5)",
            params![
                id.to_string(),
                title,
                Utc::now().to_rfc3339(),
                enc(&MeetingState::Recording),
                enc(&mode)
            ],
        )?;
        self.set_participants(id, participants)?;
        self.get_meeting(id)
    }

    /// Leaving `Recording` stamps `ended_at` once. `error` is only kept for `Failed`.
    pub fn set_state(&self, id: Uuid, state: MeetingState, error: Option<&str>) -> Result<()> {
        let ended = (state != MeetingState::Recording).then(|| Utc::now().to_rfc3339());
        let error = if state == MeetingState::Failed {
            error
        } else {
            None
        };
        let n = self.conn.execute(
            "UPDATE meetings SET state = ?2, error = ?3, ended_at = COALESCE(ended_at, ?4) WHERE id = ?1",
            params![id.to_string(), enc(&state), error, ended],
        )?;
        anyhow::ensure!(n == 1, "meeting_not_found");
        Ok(())
    }

    pub fn set_language(&self, id: Uuid, language: &str) -> Result<()> {
        self.conn.execute(
            "UPDATE meetings SET language = ?2 WHERE id = ?1",
            params![id.to_string(), language],
        )?;
        Ok(())
    }

    pub fn set_providers(&self, id: Uuid, providers: &BTreeMap<String, String>) -> Result<()> {
        self.conn.execute(
            "UPDATE meetings SET providers = ?2 WHERE id = ?1",
            params![id.to_string(), serde_json::to_string(providers)?],
        )?;
        Ok(())
    }

    pub fn get_meeting(&self, id: Uuid) -> Result<Meeting> {
        self.conn
            .query_row(
                &format!("{MEETING_SELECT} WHERE m.id = ?1"),
                [id.to_string()],
                meeting_row,
            )
            .optional()?
            .context("meeting_not_found")
    }

    /// Newest first.
    pub fn list_meetings(&self) -> Result<Vec<Meeting>> {
        let mut stmt = self
            .conn
            .prepare(&format!("{MEETING_SELECT} ORDER BY m.started_at DESC"))?;
        let rows = stmt.query_map([], meeting_row)?;
        Ok(rows.collect::<rusqlite::Result<_>>()?)
    }

    /// Meetings in any of `states`, oldest first (resume order).
    pub fn list_meetings_in(&self, states: &[MeetingState]) -> Result<Vec<Meeting>> {
        let mut all = self.list_meetings()?;
        all.retain(|m| states.contains(&m.state));
        all.reverse();
        Ok(all)
    }

    /// Deletes every row of the meeting (cascade). Audio files: `retention::delete_meeting`.
    pub fn delete_meeting(&self, id: Uuid) -> Result<()> {
        self.conn
            .execute("DELETE FROM meetings WHERE id = ?1", [id.to_string()])?;
        Ok(())
    }

    // ---------- participants ----------

    pub fn participants(&self, id: Uuid) -> Result<Vec<Participant>> {
        let mut stmt = self
            .conn
            .prepare("SELECT id, name, email, confirmed FROM participants WHERE meeting_id = ?1 ORDER BY pos")?;
        let rows = stmt.query_map([id.to_string()], |r| {
            Ok(Participant {
                id: r.get(0)?,
                name: r.get(1)?,
                email: r.get(2)?,
                confirmed: r.get(3)?,
            })
        })?;
        Ok(rows.collect::<rusqlite::Result<_>>()?)
    }

    pub fn set_participants(&self, id: Uuid, participants: &[Participant]) -> Result<()> {
        let tx = self.conn.unchecked_transaction()?;
        tx.execute(
            "DELETE FROM participants WHERE meeting_id = ?1",
            [id.to_string()],
        )?;
        for (pos, p) in participants.iter().enumerate() {
            tx.execute(
                "INSERT INTO participants (meeting_id, pos, id, name, email, confirmed) VALUES (?1, ?2, ?3, ?4, ?5, ?6)",
                params![id.to_string(), pos as i64, p.id, p.name, p.email, p.confirmed],
            )?;
        }
        tx.commit()?;
        Ok(())
    }

    /// Names a speaker of this meeting and remembers the spelling for later meetings (spec 13).
    pub fn rename_speaker(&self, id: Uuid, speaker_id: &str, name: &str) -> Result<()> {
        let mut participants = self.participants(id)?;
        match participants.iter_mut().find(|p| p.id == speaker_id) {
            Some(p) => {
                p.name = name.to_string();
                p.confirmed = true;
            }
            None => participants.push(Participant {
                id: speaker_id.into(),
                name: name.into(),
                email: None,
                confirmed: true,
            }),
        }
        self.set_participants(id, &participants)?;
        let key = if speaker_id == "me" {
            ME_KEY.to_string()
        } else {
            normalize_name(name)
        };
        self.conn.execute(
            "INSERT INTO speaker_names (key, name) VALUES (?1, ?2) ON CONFLICT(key) DO UPDATE SET name = excluded.name",
            params![key, name],
        )?;
        Ok(())
    }

    /// Remembered spelling for a participant name, if the user corrected it before.
    pub fn remembered_name(&self, name: &str) -> Result<Option<String>> {
        self.speaker_name(&normalize_name(name))
    }

    /// The user's own name, if they renamed "me" before.
    pub fn remembered_me(&self) -> Result<Option<String>> {
        self.speaker_name(ME_KEY)
    }

    fn speaker_name(&self, key: &str) -> Result<Option<String>> {
        Ok(self
            .conn
            .query_row(
                "SELECT name FROM speaker_names WHERE key = ?1",
                [key],
                |r| r.get(0),
            )
            .optional()?)
    }

    // ---------- notes ----------

    pub fn add_note(&self, id: Uuid, at: f64, text: &str, source: &str) -> Result<Note> {
        let note = Note {
            id: Uuid::new_v4().to_string(),
            at,
            text: text.into(),
            source: source.into(),
        };
        self.conn.execute(
            "INSERT INTO notes (meeting_id, id, at, text, source) VALUES (?1, ?2, ?3, ?4, ?5)",
            params![id.to_string(), note.id, note.at, note.text, note.source],
        )?;
        Ok(note)
    }

    pub fn notes(&self, id: Uuid) -> Result<Vec<Note>> {
        let mut stmt = self
            .conn
            .prepare("SELECT id, at, text, source FROM notes WHERE meeting_id = ?1 ORDER BY at")?;
        let rows = stmt.query_map([id.to_string()], |r| {
            Ok(Note {
                id: r.get(0)?,
                at: r.get(1)?,
                text: r.get(2)?,
                source: r.get(3)?,
            })
        })?;
        Ok(rows.collect::<rusqlite::Result<_>>()?)
    }

    // ---------- steps ----------

    pub fn save_step(&self, id: Uuid, step: Step, output: &serde_json::Value) -> Result<()> {
        self.conn.execute(
            "INSERT OR REPLACE INTO steps (meeting_id, step, output) VALUES (?1, ?2, ?3)",
            params![id.to_string(), enc(&step), output.to_string()],
        )?;
        Ok(())
    }

    pub fn load_step(&self, id: Uuid, step: Step) -> Result<Option<serde_json::Value>> {
        let out: Option<String> = self
            .conn
            .query_row(
                "SELECT output FROM steps WHERE meeting_id = ?1 AND step = ?2",
                params![id.to_string(), enc(&step)],
                |r| r.get(0),
            )
            .optional()?;
        Ok(out.map(|s| serde_json::from_str(&s)).transpose()?)
    }

    /// Forgets all step outputs so the next run reprocesses from scratch.
    pub fn clear_steps(&self, id: Uuid) -> Result<()> {
        self.conn
            .execute("DELETE FROM steps WHERE meeting_id = ?1", [id.to_string()])?;
        Ok(())
    }

    // ---------- segments ----------

    /// Replaces all segments of the meeting.
    pub fn save_segments(&self, id: Uuid, segments: &[Segment]) -> Result<()> {
        let tx = self.conn.unchecked_transaction()?;
        tx.execute(
            "DELETE FROM segments WHERE meeting_id = ?1",
            [id.to_string()],
        )?;
        for s in segments {
            tx.execute(
                "INSERT INTO segments (meeting_id, id, start, end, speaker_id, text, confidence, reviewed)
                 VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7, ?8)",
                params![id.to_string(), s.id, s.start, s.end, s.speaker_id, s.text, s.confidence, s.reviewed],
            )?;
        }
        tx.commit()?;
        Ok(())
    }

    pub fn segments(&self, id: Uuid) -> Result<Vec<Segment>> {
        let mut stmt = self.conn.prepare(
            "SELECT id, start, end, speaker_id, text, confidence, reviewed FROM segments WHERE meeting_id = ?1 ORDER BY start, id",
        )?;
        let rows = stmt.query_map([id.to_string()], |r| {
            Ok(Segment {
                id: r.get(0)?,
                start: r.get(1)?,
                end: r.get(2)?,
                speaker_id: r.get(3)?,
                text: r.get(4)?,
                confidence: r.get(5)?,
                reviewed: r.get(6)?,
            })
        })?;
        Ok(rows.collect::<rusqlite::Result<_>>()?)
    }

    /// A user edit counts as a review of the segment.
    pub fn update_segment_text(&self, id: Uuid, segment_id: &str, text: &str) -> Result<()> {
        self.update_segment(id, segment_id, "text", text)
    }

    pub fn update_segment_speaker(
        &self,
        id: Uuid,
        segment_id: &str,
        speaker_id: &str,
    ) -> Result<()> {
        self.update_segment(id, segment_id, "speaker_id", speaker_id)
    }

    pub fn mark_segment_reviewed(&self, id: Uuid, segment_id: &str) -> Result<()> {
        let n = self.conn.execute(
            "UPDATE segments SET reviewed = 1 WHERE meeting_id = ?1 AND id = ?2",
            params![id.to_string(), segment_id],
        )?;
        anyhow::ensure!(n == 1, "segment_not_found");
        Ok(())
    }

    fn update_segment(
        &self,
        id: Uuid,
        segment_id: &str,
        column: &'static str,
        value: &str,
    ) -> Result<()> {
        let n = self.conn.execute(
            &format!(
                "UPDATE segments SET {column} = ?3, reviewed = 1 WHERE meeting_id = ?1 AND id = ?2"
            ),
            params![id.to_string(), segment_id, value],
        )?;
        anyhow::ensure!(n == 1, "segment_not_found");
        Ok(())
    }

    // ---------- review items ----------

    /// Replaces all review items of the meeting.
    pub fn set_review_items(&self, id: Uuid, items: &[ReviewItem]) -> Result<()> {
        let tx = self.conn.unchecked_transaction()?;
        tx.execute(
            "DELETE FROM review_items WHERE meeting_id = ?1",
            [id.to_string()],
        )?;
        for i in items {
            tx.execute(
                "INSERT INTO review_items (id, meeting_id, segment_id, kind, start, end, proposed_text, alternatives, question, resolution)
                 VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7, ?8, ?9, ?10)",
                params![
                    i.id,
                    id.to_string(),
                    i.segment_id,
                    enc(&i.kind),
                    i.start,
                    i.end,
                    i.proposed_text,
                    serde_json::to_string(&i.alternatives)?,
                    i.question,
                    i.resolution.as_ref().map(serde_json::to_string).transpose()?,
                ],
            )?;
        }
        tx.commit()?;
        Ok(())
    }

    pub fn review_items(&self, id: Uuid) -> Result<Vec<ReviewItem>> {
        let mut stmt = self.conn.prepare(&format!(
            "{REVIEW_SELECT} WHERE meeting_id = ?1 ORDER BY start, id"
        ))?;
        let rows = stmt.query_map([id.to_string()], review_row)?;
        Ok(rows.collect::<rusqlite::Result<_>>()?)
    }

    pub fn get_review_item(&self, item_id: &str) -> Result<ReviewItem> {
        self.conn
            .query_row(
                &format!("{REVIEW_SELECT} WHERE id = ?1"),
                [item_id],
                review_row,
            )
            .optional()?
            .context("review_item_not_found")
    }

    pub fn set_review_resolution(&self, item_id: &str, resolution: &Resolution) -> Result<()> {
        let n = self.conn.execute(
            "UPDATE review_items SET resolution = ?2 WHERE id = ?1",
            params![item_id, serde_json::to_string(resolution)?],
        )?;
        anyhow::ensure!(n == 1, "review_item_not_found");
        Ok(())
    }

    // ---------- assembled document ----------

    /// `summary` comes from the `structure` step output (null when absent or disabled).
    pub fn meeting_doc(&self, id: Uuid) -> Result<MeetingDoc> {
        let m = self.get_meeting(id)?;
        let providers: String = self.conn.query_row(
            "SELECT providers FROM meetings WHERE id = ?1",
            [id.to_string()],
            |r| r.get(0),
        )?;
        let summary = match self.load_step(id, Step::Structure)? {
            Some(v) => serde_json::from_value(v)?,
            None => None,
        };
        Ok(MeetingDoc {
            schema_version: SCHEMA_VERSION.into(),
            meeting: MeetingInfo {
                id,
                title: m.title,
                started_at: m.started_at,
                ended_at: m.ended_at,
                language: m.language,
                participants: self.participants(id)?,
            },
            segments: self.segments(id)?,
            notes: self.notes(id)?,
            summary,
            processing: Processing {
                mode: m.mode,
                providers: serde_json::from_str(&providers)?,
                review_items_total: m.review_items_total,
                review_items_resolved: m.review_items_resolved,
            },
        })
    }

    // ---------- dictations ----------

    pub fn add_dictation(&self, d: &Dictation) -> Result<()> {
        self.conn.execute(
            "INSERT INTO dictations (id, created_at, text, language, duration_ms, injected_into, provider)
             VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7)",
            params![d.id.to_string(), d.created_at.to_rfc3339(), d.text, d.language, d.duration_ms as i64, d.injected_into, d.provider],
        )?;
        Ok(())
    }

    /// Newest first.
    pub fn list_dictations(&self) -> Result<Vec<Dictation>> {
        let mut stmt = self.conn.prepare(
            "SELECT id, created_at, text, language, duration_ms, injected_into, provider FROM dictations ORDER BY created_at DESC",
        )?;
        let rows = stmt.query_map([], |r| {
            Ok(Dictation {
                id: parse_uuid(r, 0)?,
                created_at: parse_time(r, 1)?,
                text: r.get(2)?,
                language: r.get(3)?,
                duration_ms: r.get::<_, i64>(4)? as u64,
                injected_into: r.get(5)?,
                provider: r.get(6)?,
            })
        })?;
        Ok(rows.collect::<rusqlite::Result<_>>()?)
    }

    pub fn delete_dictation(&self, id: Uuid) -> Result<()> {
        self.conn
            .execute("DELETE FROM dictations WHERE id = ?1", [id.to_string()])?;
        Ok(())
    }

    pub fn clear_dictations(&self) -> Result<()> {
        self.conn.execute("DELETE FROM dictations", [])?;
        Ok(())
    }

    // ---------- settings ----------

    pub fn settings(&self) -> Result<Settings> {
        let json: Option<String> = self
            .conn
            .query_row("SELECT json FROM settings WHERE id = 1", [], |r| r.get(0))
            .optional()?;
        Ok(json
            .map(|j| serde_json::from_str(&j))
            .transpose()?
            .unwrap_or_default())
    }

    pub fn save_settings(&self, settings: &Settings) -> Result<()> {
        self.conn.execute(
            "INSERT OR REPLACE INTO settings (id, json) VALUES (1, ?1)",
            [serde_json::to_string(settings)?],
        )?;
        Ok(())
    }
}

/// Case- and whitespace-insensitive key for remembered names.
pub fn normalize_name(name: &str) -> String {
    name.split_whitespace()
        .collect::<Vec<_>>()
        .join(" ")
        .to_lowercase()
}

const MEETING_SELECT: &str =
    "SELECT m.id, m.title, m.started_at, m.ended_at, m.language, m.state, m.error, m.mode,
    (SELECT COUNT(*) FROM review_items r WHERE r.meeting_id = m.id),
    (SELECT COUNT(*) FROM review_items r WHERE r.meeting_id = m.id AND r.resolution IS NOT NULL)
    FROM meetings m";

fn meeting_row(r: &Row) -> rusqlite::Result<Meeting> {
    Ok(Meeting {
        id: parse_uuid(r, 0)?,
        title: r.get(1)?,
        started_at: parse_time(r, 2)?,
        ended_at: r
            .get::<_, Option<String>>(3)?
            .map(|s| parse_rfc3339(&s, 3))
            .transpose()?,
        language: r.get(4)?,
        state: dec(r, 5)?,
        error: r.get(6)?,
        mode: dec(r, 7)?,
        review_items_total: r.get(8)?,
        review_items_resolved: r.get(9)?,
    })
}

const REVIEW_SELECT: &str = "SELECT id, meeting_id, segment_id, kind, start, end, proposed_text, alternatives, question, resolution FROM review_items";

fn review_row(r: &Row) -> rusqlite::Result<ReviewItem> {
    let resolution: Option<String> = r.get(9)?;
    Ok(ReviewItem {
        id: r.get(0)?,
        meeting_id: parse_uuid(r, 1)?,
        segment_id: r.get(2)?,
        kind: dec(r, 3)?,
        start: r.get(4)?,
        end: r.get(5)?,
        proposed_text: r.get(6)?,
        alternatives: json(&r.get::<_, String>(7)?, 7)?,
        question: r.get(8)?,
        resolution: resolution.map(|s| json(&s, 9)).transpose()?,
    })
}

fn json<T: DeserializeOwned>(s: &str, i: usize) -> rusqlite::Result<T> {
    serde_json::from_str(s).map_err(|e| conv_err(i, e))
}

/// Serializes a unit enum to its snake_case name.
fn enc<T: Serialize>(v: &T) -> String {
    match serde_json::to_value(v) {
        Ok(serde_json::Value::String(s)) => s,
        _ => unreachable!("only used with unit enums"),
    }
}

fn dec<T: DeserializeOwned>(r: &Row, i: usize) -> rusqlite::Result<T> {
    let s: String = r.get(i)?;
    serde_json::from_value(serde_json::Value::String(s)).map_err(|e| conv_err(i, e))
}

fn parse_uuid(r: &Row, i: usize) -> rusqlite::Result<Uuid> {
    let s: String = r.get(i)?;
    Uuid::parse_str(&s).map_err(|e| conv_err(i, e))
}

fn parse_time(r: &Row, i: usize) -> rusqlite::Result<DateTime<Utc>> {
    parse_rfc3339(&r.get::<_, String>(i)?, i)
}

fn parse_rfc3339(s: &str, i: usize) -> rusqlite::Result<DateTime<Utc>> {
    DateTime::parse_from_rfc3339(s)
        .map(|t| t.with_timezone(&Utc))
        .map_err(|e| conv_err(i, e))
}

fn conv_err(i: usize, e: impl std::error::Error + Send + Sync + 'static) -> rusqlite::Error {
    rusqlite::Error::FromSqlConversionFailure(i, rusqlite::types::Type::Text, Box::new(e))
}

#[cfg(test)]
mod tests {
    use super::*;

    fn p(id: &str, name: &str) -> Participant {
        Participant {
            id: id.into(),
            name: name.into(),
            email: None,
            confirmed: true,
        }
    }

    fn seg(id: &str, start: f64) -> Segment {
        Segment {
            id: id.into(),
            start,
            end: start + 1.0,
            speaker_id: None,
            text: "t".into(),
            confidence: 0.9,
            reviewed: false,
        }
    }

    #[test]
    fn migrations_run_once_and_data_persists() {
        let dir = std::env::temp_dir().join(format!("barnaby-store-{}", Uuid::new_v4()));
        std::fs::create_dir_all(&dir).unwrap();
        let path = dir.join("barnaby.db");
        let id = {
            let s = Store::open(&path).unwrap();
            assert_eq!(s.schema_version().unwrap(), MIGRATIONS.len());
            s.create_meeting("Weekly", &[p("p1", "Alice")], ProcessingMode::Local)
                .unwrap()
                .id
        };
        let s = Store::open(&path).unwrap();
        assert_eq!(s.schema_version().unwrap(), MIGRATIONS.len());
        assert_eq!(s.get_meeting(id).unwrap().title, "Weekly");
        std::fs::remove_dir_all(dir).unwrap();
    }

    #[test]
    fn meeting_crud_and_doc() {
        let s = Store::open_in_memory().unwrap();
        let m = s
            .create_meeting("A", &[p("p1", "Alice")], ProcessingMode::Local)
            .unwrap();
        assert_eq!(m.state, MeetingState::Recording);
        assert!(m.ended_at.is_none());
        let m2 = s.create_meeting("B", &[], ProcessingMode::Local).unwrap();
        let listed: Vec<_> = s
            .list_meetings()
            .unwrap()
            .into_iter()
            .map(|m| m.id)
            .collect();
        assert_eq!(listed, vec![m2.id, m.id]);

        s.set_state(m.id, MeetingState::Failed, Some("x_failed"))
            .unwrap();
        let got = s.get_meeting(m.id).unwrap();
        assert_eq!(
            (got.state, got.error.as_deref()),
            (MeetingState::Failed, Some("x_failed"))
        );
        assert!(got.ended_at.is_some());
        s.set_state(m.id, MeetingState::Processing, None).unwrap();
        assert!(s.get_meeting(m.id).unwrap().error.is_none());
        assert_eq!(
            s.list_meetings_in(&[MeetingState::Processing])
                .unwrap()
                .len(),
            1
        );

        s.add_note(m.id, 3.0, "n", "manual").unwrap();
        s.save_segments(m.id, &[seg("s2", 5.0), seg("s1", 1.0)])
            .unwrap();
        s.save_segments(m.id, &[seg("s2", 5.0), seg("s1", 1.0)])
            .unwrap();
        s.update_segment_text(m.id, "s1", "fixed").unwrap();
        s.update_segment_speaker(m.id, "s2", "p1").unwrap();
        assert!(s.update_segment_text(m.id, "nope", "x").is_err());
        s.save_step(
            m.id,
            Step::Structure,
            &serde_json::to_value(Summary::default()).unwrap(),
        )
        .unwrap();

        let doc = s.meeting_doc(m.id).unwrap();
        assert_eq!(doc.segments.len(), 2);
        assert_eq!(
            (doc.segments[0].text.as_str(), doc.segments[0].reviewed),
            ("fixed", true)
        );
        assert_eq!(doc.segments[1].speaker_id.as_deref(), Some("p1"));
        assert_eq!(doc.notes.len(), 1);
        assert_eq!(doc.summary, Some(Summary::default()));
        assert_eq!(doc.meeting.participants, vec![p("p1", "Alice")]);

        s.clear_steps(m.id).unwrap();
        assert!(s.load_step(m.id, Step::Structure).unwrap().is_none());

        s.delete_meeting(m.id).unwrap();
        assert!(s.get_meeting(m.id).is_err());
        assert!(s.segments(m.id).unwrap().is_empty());
        assert!(s.notes(m.id).unwrap().is_empty());
    }

    #[test]
    fn rename_speaker_is_remembered() {
        let s = Store::open_in_memory().unwrap();
        let m = s
            .create_meeting("A", &[p("me", "Moi")], ProcessingMode::Local)
            .unwrap();
        s.rename_speaker(m.id, "speaker_a", "Jean-François  Dupont")
            .unwrap();
        s.rename_speaker(m.id, "me", "Jon").unwrap();
        let ps = s.participants(m.id).unwrap();
        assert_eq!(ps[1].name, "Jean-François  Dupont");
        assert_eq!(ps[0].name, "Jon");
        assert_eq!(
            s.remembered_name("jean-françois dupont")
                .unwrap()
                .as_deref(),
            Some("Jean-François  Dupont")
        );
        assert_eq!(s.remembered_me().unwrap().as_deref(), Some("Jon"));
    }

    #[test]
    fn dictations_and_settings() {
        let s = Store::open_in_memory().unwrap();
        assert_eq!(s.settings().unwrap(), Settings::default());
        let mut st = Settings::default();
        st.meeting.confidence_threshold = 0.4;
        s.save_settings(&st).unwrap();
        assert_eq!(s.settings().unwrap(), st);

        let d = |t: i64| Dictation {
            id: Uuid::new_v4(),
            created_at: DateTime::from_timestamp(t, 0).unwrap(),
            text: "x".into(),
            language: Some("fr".into()),
            duration_ms: 10,
            injected_into: None,
            provider: "w".into(),
        };
        let (a, b) = (d(1), d(2));
        s.add_dictation(&a).unwrap();
        s.add_dictation(&b).unwrap();
        assert_eq!(s.list_dictations().unwrap(), vec![b.clone(), a.clone()]);
        s.delete_dictation(b.id).unwrap();
        assert_eq!(s.list_dictations().unwrap(), vec![a]);
        s.clear_dictations().unwrap();
        assert!(s.list_dictations().unwrap().is_empty());
    }
}
