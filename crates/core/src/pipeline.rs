//! Meeting processing: `Step::ALL` in order, each output persisted so a rerun
//! skips finished steps (architecture invariant 2).

use std::collections::BTreeMap;
use std::path::PathBuf;
use std::sync::{Arc, MutexGuard};

use anyhow::{bail, Result};
use serde_json::json;
use uuid::Uuid;

use crate::audio::{mix, read_wav, repair_wav, rms, slice};
use crate::model::*;
use crate::provider::*;
use crate::retention::{meeting_dir, MIC_WAV, SYSTEM_WAV};
use crate::store::{lock, SharedStore, Store};
use crate::{net, review};

pub struct Pipeline {
    pub store: SharedStore,
    pub transcriber: Arc<dyn Transcriber>,
    pub diarizer: Arc<dyn Diarizer>,
    /// None = LLM disabled; `summary` stays null.
    pub structurer: Option<Arc<dyn Structurer>>,
    pub data_dir: PathBuf,
}

#[derive(Default)]
struct Tracks {
    mic: Vec<f32>,
    system: Vec<f32>,
    mixed: Vec<f32>,
}

type Failure = (String, anyhow::Error);

impl Pipeline {
    /// The store lock is only taken for short reads/writes, never around a provider call.
    fn db(&self) -> MutexGuard<'_, Store> {
        lock(&self.store)
    }

    /// Processing → NeedsReview, or Failed with a short code (no content).
    pub fn run(&self, id: Uuid) -> Result<()> {
        {
            let db = self.db();
            db.set_state(id, MeetingState::Processing, None)?;
            let mut providers = BTreeMap::new();
            providers.insert(
                "transcription".to_string(),
                self.transcriber.name().to_string(),
            );
            providers.insert("diarization".to_string(), self.diarizer.name().to_string());
            if let Some(s) = &self.structurer {
                providers.insert("structuring".to_string(), s.name().to_string());
            }
            db.set_providers(id, &providers)?;
        }
        match self.process(id) {
            Ok(()) => {
                self.db().set_state(id, MeetingState::NeedsReview, None)?;
                log::info!("meeting {id} processed");
                Ok(())
            }
            Err((code, err)) => {
                log::warn!("meeting {id} failed: {code}");
                self.db().set_state(id, MeetingState::Failed, Some(&code))?;
                Err(err)
            }
        }
    }

    /// Runs every meeting left captured, queued or mid-processing (app start). Returns their ids.
    pub fn resume_pending(&self) -> Result<Vec<Uuid>> {
        let pending = self.db().list_meetings_in(&[
            MeetingState::Captured,
            MeetingState::Queued,
            MeetingState::Processing,
        ])?;
        // Failures are recorded on each meeting; one bad meeting must not block the others.
        Ok(pending
            .into_iter()
            .map(|m| m.id)
            .inspect(|id| drop(self.run(*id)))
            .collect())
    }

    fn process(&self, id: Uuid) -> Result<(), Failure> {
        let (settings, pending) = {
            let db = self.db();
            let settings = db.settings().map_err(fail("store_failed"))?;
            let mut pending = Vec::new();
            for step in Step::ALL {
                if db
                    .load_step(id, step)
                    .map_err(fail("store_failed"))?
                    .is_none()
                {
                    pending.push(step);
                }
            }
            (settings, pending)
        };
        let tracks = if pending.iter().any(|s| *s != Step::Structure) {
            self.load_audio(id).map_err(fail("audio_failed"))?
        } else {
            Tracks::default()
        };
        for step in pending {
            self.step(id, step, &tracks, &settings).map_err(|e| {
                let code = if net::is_blocked(&e) {
                    net::BLOCKED.to_string()
                } else {
                    format!("{step:?}_failed").to_lowercase()
                };
                (code, e)
            })?;
        }
        Ok(())
    }

    fn load_audio(&self, id: Uuid) -> Result<Tracks> {
        let dir = meeting_dir(&self.data_dir, id);
        let read = |name: &str| -> Result<Vec<f32>> {
            let path = dir.join(name);
            if !path.exists() {
                return Ok(Vec::new());
            }
            repair_wav(&path)?;
            read_wav(&path)
        };
        let (mic, system) = (read(MIC_WAV)?, read(SYSTEM_WAV)?);
        if mic.is_empty() && system.is_empty() {
            bail!("audio_missing");
        }
        Ok(Tracks {
            mixed: mix(&mic, &system),
            mic,
            system,
        })
    }

    fn step(&self, id: Uuid, step: Step, t: &Tracks, settings: &Settings) -> Result<()> {
        let output = match step {
            Step::Transcribe => {
                let lang = Some(settings.transcription_language.as_str()).filter(|l| *l != "auto");
                let transcript = self.transcriber.transcribe(&t.mixed, lang)?;
                self.db().set_language(id, &transcript.language)?;
                serde_json::to_value(transcript)?
            }
            Step::Diarize => serde_json::to_value(self.diarizer.diarize(&t.mixed)?)?,
            Step::Identify => {
                let db = self.db();
                let transcript: Transcript = load(&db, id, Step::Transcribe)?;
                let turns: Vec<SpeakerTurn> = load(&db, id, Step::Diarize)?;
                let labels = label_turns(&turns, &t.mic, &t.system);
                let segments = assign_segments(&transcript, &turns, &labels);
                let participants = merge_participants(&db, db.participants(id)?, &labels)?;
                db.set_participants(id, &participants)?;
                db.save_segments(id, &segments)?;
                json!({ "segments": segments.len() })
            }
            Step::Review => {
                let db = self.db();
                let items = review::detect(
                    id,
                    &db.segments(id)?,
                    &t.mic,
                    &t.system,
                    settings.meeting.confidence_threshold,
                );
                db.set_review_items(id, &items)?;
                json!({ "items": items.len() })
            }
            Step::Structure => match &self.structurer {
                Some(s) => {
                    let doc = self.db().meeting_doc(id)?;
                    serde_json::to_value(Some(s.structure(&doc)?))?
                }
                None => serde_json::Value::Null,
            },
        };
        self.db().save_step(id, step, &output)
    }
}

fn fail(code: &'static str) -> impl Fn(anyhow::Error) -> Failure {
    move |e| (code.to_string(), e)
}

fn load<T: serde::de::DeserializeOwned>(db: &Store, id: Uuid, step: Step) -> Result<T> {
    match db.load_step(id, step)? {
        Some(v) => Ok(serde_json::from_value(v)?),
        None => bail!("missing_step_output"),
    }
}

/// Speaker id per turn: "me" when the mic track dominates the turn, otherwise
/// "speaker_a", "speaker_b"... by first appearance of the diarizer's index.
pub fn label_turns(turns: &[SpeakerTurn], mic: &[f32], system: &[f32]) -> Vec<String> {
    let mut others: Vec<u32> = Vec::new();
    turns
        .iter()
        .map(|t| {
            if rms(slice(mic, t.start, t.end)) > rms(slice(system, t.start, t.end)) {
                return "me".to_string();
            }
            let idx = others
                .iter()
                .position(|s| *s == t.speaker)
                .unwrap_or_else(|| {
                    others.push(t.speaker);
                    others.len() - 1
                });
            format!("speaker_{}", letter(idx).to_ascii_lowercase())
        })
        .collect()
}

fn letter(idx: usize) -> String {
    match u8::try_from(idx) {
        Ok(i) if i < 26 => char::from(b'A' + i).to_string(),
        _ => (idx + 1).to_string(),
    }
}

/// Each segment takes the label of the turn it overlaps most (none if no overlap).
pub fn assign_segments(
    transcript: &Transcript,
    turns: &[SpeakerTurn],
    labels: &[String],
) -> Vec<Segment> {
    transcript
        .segments
        .iter()
        .enumerate()
        .map(|(i, s)| {
            let speaker = turns
                .iter()
                .zip(labels)
                .map(|(t, l)| (s.end.min(t.end) - s.start.max(t.start), l))
                .filter(|(overlap, _)| *overlap > 0.0)
                .max_by(|a, b| a.0.total_cmp(&b.0))
                .map(|(_, l)| l.clone());
            Segment {
                id: format!("s{}", i + 1),
                start: s.start,
                end: s.end,
                speaker_id: speaker,
                text: s.text.clone(),
                confidence: s.confidence,
                reviewed: false,
            }
        })
        .collect()
}

/// Adds the user ("me") and anonymous speakers, and applies remembered spellings
/// to names the user typed. Names stay suggestions: nothing is matched by voice.
fn merge_participants(
    db: &Store,
    mut participants: Vec<Participant>,
    labels: &[String],
) -> Result<Vec<Participant>> {
    for p in &mut participants {
        if let Some(name) = db.remembered_name(&p.name)? {
            p.name = name;
        }
    }
    if !participants.iter().any(|p| p.id == "me") {
        let name = db.remembered_me()?.unwrap_or_else(|| "Moi".into());
        participants.insert(
            0,
            Participant {
                id: "me".into(),
                name,
                email: None,
                confirmed: true,
            },
        );
    }
    for label in labels {
        if !participants.iter().any(|p| &p.id == label) {
            let name = format!(
                "Speaker {}",
                label.trim_start_matches("speaker_").to_uppercase()
            );
            participants.push(Participant {
                id: label.clone(),
                name,
                email: None,
                confirmed: false,
            });
        }
    }
    Ok(participants)
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::audio::write_test_wav;
    use crate::destination::tests::serve;
    use crate::llm::{tests::chat_reply, OpenAiStructurer};
    use crate::net::NetPolicy;
    use std::sync::atomic::{AtomicBool, AtomicUsize, Ordering};
    use std::sync::Mutex;

    #[derive(Default)]
    struct FakeTranscriber(AtomicUsize);
    impl Transcriber for FakeTranscriber {
        fn name(&self) -> &str {
            "fake-asr"
        }
        fn transcribe(&self, _: &[f32], _: Option<&str>) -> Result<Transcript> {
            self.0.fetch_add(1, Ordering::SeqCst);
            Ok(Transcript {
                language: "fr".into(),
                segments: vec![
                    RawSegment {
                        start: 0.0,
                        end: 1.9,
                        text: "bonjour".into(),
                        confidence: 0.9,
                    },
                    RawSegment {
                        start: 2.1,
                        end: 4.0,
                        text: "salut".into(),
                        confidence: 0.3,
                    },
                ],
            })
        }
    }

    struct FakeDiarizer;
    impl Diarizer for FakeDiarizer {
        fn name(&self) -> &str {
            "fake-diar"
        }
        fn diarize(&self, _: &[f32]) -> Result<Vec<SpeakerTurn>> {
            Ok(vec![
                SpeakerTurn {
                    start: 0.0,
                    end: 2.0,
                    speaker: 0,
                },
                SpeakerTurn {
                    start: 2.0,
                    end: 4.0,
                    speaker: 1,
                },
            ])
        }
    }

    #[derive(Default)]
    struct FlakyStructurer(AtomicBool);
    impl Structurer for FlakyStructurer {
        fn name(&self) -> &str {
            "fake-llm"
        }
        fn structure(&self, _: &MeetingDoc) -> Result<Summary> {
            if self.0.swap(true, Ordering::SeqCst) {
                Ok(Summary {
                    overview: "ok".into(),
                    ..Default::default()
                })
            } else {
                bail!("boom")
            }
        }
    }

    /// Mic speaks during 0..2 s, system during 2..4 s.
    fn setup(participants: &[Participant]) -> (SharedStore, PathBuf, Uuid) {
        let data_dir = std::env::temp_dir().join(format!("barnaby-pipe-{}", Uuid::new_v4()));
        let store = Store::open_in_memory().unwrap();
        let id = store
            .create_meeting("t", participants, ProcessingMode::Local)
            .unwrap()
            .id;
        store.set_state(id, MeetingState::Captured, None).unwrap();
        let dir = meeting_dir(&data_dir, id);
        std::fs::create_dir_all(&dir).unwrap();
        let mut mic = vec![0.0; 16_000 * 4];
        mic[..32_000].fill(0.3);
        let mut system = vec![0.0; 16_000 * 4];
        system[32_000..].fill(0.3);
        write_test_wav(&dir.join(MIC_WAV), &mic);
        write_test_wav(&dir.join(SYSTEM_WAV), &system);
        (Arc::new(Mutex::new(store)), data_dir, id)
    }

    fn pipeline(
        store: &SharedStore,
        data_dir: &std::path::Path,
        asr: Arc<FakeTranscriber>,
        llm: Option<Arc<dyn Structurer>>,
    ) -> Pipeline {
        Pipeline {
            store: store.clone(),
            transcriber: asr,
            diarizer: Arc::new(FakeDiarizer),
            structurer: llm,
            data_dir: data_dir.into(),
        }
    }

    #[test]
    fn identifies_speakers_and_is_idempotent() {
        let (store, dir, id) = setup(&[]);
        let asr = Arc::new(FakeTranscriber::default());
        let p = pipeline(&store, &dir, asr.clone(), None);
        p.run(id).unwrap();
        p.run(id).unwrap();

        let db = lock(&store);
        let m = db.get_meeting(id).unwrap();
        assert_eq!(m.state, MeetingState::NeedsReview);
        assert_eq!(m.language.as_deref(), Some("fr"));
        assert_eq!(asr.0.load(Ordering::SeqCst), 1);
        let doc = db.meeting_doc(id).unwrap();
        let speakers: Vec<_> = doc
            .segments
            .iter()
            .map(|s| s.speaker_id.as_deref())
            .collect();
        assert_eq!(speakers, vec![Some("me"), Some("speaker_a")]);
        let names: Vec<_> = doc
            .meeting
            .participants
            .iter()
            .map(|p| (p.id.as_str(), p.name.as_str()))
            .collect();
        assert_eq!(names, vec![("me", "Moi"), ("speaker_a", "Speaker A")]);
        assert_eq!(doc.summary, None);
        assert_eq!(doc.processing.providers["transcription"], "fake-asr");
        // Only s2 is below the confidence threshold.
        assert_eq!((m.review_items_total, m.review_items_resolved), (1, 0));

        // Full reprocess replaces rather than duplicates.
        db.clear_steps(id).unwrap();
        drop(db);
        p.run(id).unwrap();
        let db = lock(&store);
        assert_eq!(db.segments(id).unwrap().len(), 2);
        assert_eq!(db.review_items(id).unwrap().len(), 1);
        assert_eq!(db.participants(id).unwrap().len(), 2);
        std::fs::remove_dir_all(dir).unwrap();
    }

    #[test]
    fn failed_step_resumes_without_redoing_earlier_steps() {
        let (store, dir, id) = setup(&[]);
        let asr = Arc::new(FakeTranscriber::default());
        let p = pipeline(
            &store,
            &dir,
            asr.clone(),
            Some(Arc::new(FlakyStructurer::default())),
        );
        assert!(p.run(id).is_err());
        let m = lock(&store).get_meeting(id).unwrap();
        assert_eq!(
            (m.state, m.error.as_deref()),
            (MeetingState::Failed, Some("structure_failed"))
        );

        assert_eq!(
            p.resume_pending().unwrap(),
            Vec::<Uuid>::new(),
            "failed meetings wait for a manual retry"
        );
        p.run(id).unwrap();
        let db = lock(&store);
        assert_eq!(db.get_meeting(id).unwrap().state, MeetingState::NeedsReview);
        assert_eq!(asr.0.load(Ordering::SeqCst), 1);
        assert_eq!(db.meeting_doc(id).unwrap().summary.unwrap().overview, "ok");
        std::fs::remove_dir_all(dir).unwrap();
    }

    #[test]
    fn resume_pending_runs_captured_meetings() {
        let (store, dir, id) = setup(&[]);
        let p = pipeline(&store, &dir, Arc::new(FakeTranscriber::default()), None);
        assert_eq!(p.resume_pending().unwrap(), vec![id]);
        assert_eq!(
            lock(&store).get_meeting(id).unwrap().state,
            MeetingState::NeedsReview
        );
        std::fs::remove_dir_all(dir).unwrap();
    }

    #[test]
    fn remembered_names_apply_to_matching_participants() {
        let typed = Participant {
            id: "p1".into(),
            name: "jean  DUPONT".into(),
            email: None,
            confirmed: true,
        };
        let (store, dir, id) = setup(&[typed]);
        {
            let db = lock(&store);
            let other = db
                .create_meeting("old", &[], ProcessingMode::Local)
                .unwrap()
                .id;
            db.rename_speaker(other, "speaker_a", "Jean Dupont")
                .unwrap();
            db.rename_speaker(other, "me", "Jon").unwrap();
        }
        pipeline(&store, &dir, Arc::new(FakeTranscriber::default()), None)
            .run(id)
            .unwrap();
        let ps = lock(&store).participants(id).unwrap();
        let names: Vec<_> = ps
            .iter()
            .map(|p| (p.id.as_str(), p.name.as_str()))
            .collect();
        assert_eq!(
            names,
            vec![
                ("me", "Jon"),
                ("p1", "Jean Dupont"),
                ("speaker_a", "Speaker A")
            ]
        );
        std::fs::remove_dir_all(dir).unwrap();
    }

    #[test]
    fn missing_audio_fails_with_code() {
        let (store, dir, id) = setup(&[]);
        std::fs::remove_dir_all(&dir).unwrap();
        assert!(
            pipeline(&store, &dir, Arc::new(FakeTranscriber::default()), None)
                .run(id)
                .is_err()
        );
        assert_eq!(
            lock(&store).get_meeting(id).unwrap().error.as_deref(),
            Some("audio_failed")
        );
    }

    fn llm(base_url: String) -> Option<Arc<dyn Structurer>> {
        Some(Arc::new(OpenAiStructurer {
            base_url,
            model: "m".into(),
            api_key: None,
            policy: NetPolicy {
                mode: ProcessingMode::Local,
            },
        }))
    }

    /// Spec 16: no network request outside loopback while processing in local mode.
    #[test]
    fn local_mode_blocks_remote_llm_before_connecting() {
        let (store, dir, id) = setup(&[]);
        // A routable-looking address that would hang or fail differently if dialed.
        let p = pipeline(
            &store,
            &dir,
            Arc::new(FakeTranscriber::default()),
            llm("http://203.0.113.7:9/v1".into()),
        );
        let err = p.run(id).unwrap_err();
        assert!(net::is_blocked(&err));
        let m = lock(&store).get_meeting(id).unwrap();
        assert_eq!(
            (m.state, m.error.as_deref()),
            (MeetingState::Failed, Some(net::BLOCKED))
        );
        std::fs::remove_dir_all(dir).unwrap();
    }

    #[test]
    fn local_mode_reaches_loopback_llm() {
        let (store, dir, id) = setup(&[]);
        let summary = Summary {
            overview: "local".into(),
            ..Default::default()
        };
        let (url, rx) = serve(vec![(200, chat_reply(&summary))]);
        pipeline(
            &store,
            &dir,
            Arc::new(FakeTranscriber::default()),
            llm(format!("{url}/v1")),
        )
        .run(id)
        .unwrap();
        assert!(rx.recv().unwrap().1.contains("Moi: bonjour"));
        assert_eq!(lock(&store).meeting_doc(id).unwrap().summary, Some(summary));
        std::fs::remove_dir_all(dir).unwrap();
    }

    #[test]
    fn labels_and_overlap_assignment() {
        let mic = [vec![0.5; 16_000], vec![0.0; 32_000]].concat();
        let system = [vec![0.0; 16_000], vec![0.5; 32_000]].concat();
        let turns = vec![
            SpeakerTurn {
                start: 0.0,
                end: 1.0,
                speaker: 3,
            },
            SpeakerTurn {
                start: 1.0,
                end: 2.0,
                speaker: 7,
            },
            SpeakerTurn {
                start: 2.0,
                end: 3.0,
                speaker: 3,
            },
        ];
        assert_eq!(
            label_turns(&turns, &mic, &system),
            vec!["me", "speaker_a", "speaker_b"]
        );
        let transcript = Transcript {
            language: "fr".into(),
            segments: vec![
                RawSegment {
                    start: 0.8,
                    end: 1.9,
                    text: "a".into(),
                    confidence: 1.0,
                },
                RawSegment {
                    start: 5.0,
                    end: 6.0,
                    text: "b".into(),
                    confidence: 1.0,
                },
            ],
        };
        let labels = vec!["me".into(), "speaker_a".into(), "speaker_b".into()];
        let segs = assign_segments(&transcript, &turns, &labels);
        assert_eq!(segs[0].speaker_id.as_deref(), Some("speaker_a"));
        assert_eq!(segs[1].speaker_id, None);
        assert_eq!(letter(26), "27");
    }
}
