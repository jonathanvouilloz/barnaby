//! Meeting processing: `Step::ALL` in order, each output persisted so a rerun
//! skips finished steps (architecture invariant 2).

use std::collections::BTreeMap;
use std::path::PathBuf;
use std::sync::{Arc, MutexGuard};
use std::time::Instant;

use anyhow::{bail, Result};
use serde_json::json;
use uuid::Uuid;

use crate::audio::{mix, read_wav, repair_wav, rms, slice, write_wav};
use crate::model::*;
use crate::provider::*;
use crate::retention::{meeting_dir, MIC_WAV, MIX_WAV, SYSTEM_WAV};
use crate::store::{lock, SharedStore, Store};
use crate::{net, review, vocabulary};

pub struct Pipeline {
    pub store: SharedStore,
    pub transcriber: Arc<dyn Transcriber>,
    pub diarizer: Arc<dyn Diarizer>,
    /// None = LLM disabled; `summary` stays null.
    pub structurer: Option<Arc<dyn Structurer>>,
    pub data_dir: PathBuf,
    /// Called at the start and end of each step, and within the long ones.
    pub progress: Option<Arc<dyn Fn(Progress) + Send + Sync>>,
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
            let t = self.load_audio(id).map_err(fail("audio_failed"))?;
            log::info!("meeting {id} audio {:.0}s", t.mixed.len() as f64 / f64::from(SAMPLE_RATE));
            t
        } else {
            Tracks::default()
        };
        let total = if self.structurer.is_some() { 5 } else { 4 };
        for step in Step::ALL.into_iter().take(total) {
            let report = self.reporter(id, step, total as u8);
            if !pending.contains(&step) {
                report(1.0);
                continue;
            }
            report(0.0);
            log::info!("meeting {id} step {step:?} started");
            let started = Instant::now();
            let result = self.step(id, step, &tracks, &settings, &report);
            if result.is_ok() {
                log::info!("meeting {id} step {step:?} done in {:.1}s", started.elapsed().as_secs_f64());
                report(1.0);
            }
            match result {
                Ok(()) => {}
                // The summary is optional: an unreachable LLM must not block the review.
                // Nothing is saved, so the next run tries the summary again.
                Err(e) if step == Step::Structure && !net::is_blocked(&e) => {
                    log::warn!("meeting {id}: structure_failed");
                }
                Err(e) => {
                    let code = if net::is_blocked(&e) {
                        net::BLOCKED.to_string()
                    } else if let Some(code) = error_code(&e) {
                        code.to_string()
                    } else {
                        format!("{step:?}_failed").to_lowercase()
                    };
                    return Err((code, e));
                }
            }
        }
        // Without a structurer the summary step only stores null: done silently.
        if total == 4 && pending.contains(&Step::Structure) {
            self.step(id, Step::Structure, &tracks, &settings, &self.reporter(id, Step::Structure, 4))
                .map_err(fail("structure_failed"))?;
        }
        Ok(())
    }

    /// Progress of `step` as a `ProgressFn`; a no-op without `self.progress`.
    fn reporter(&self, meeting_id: Uuid, step: Step, total: u8) -> ProgressFn {
        let Some(sink) = self.progress.clone() else { return Arc::new(|_| {}) };
        let index = Step::ALL.iter().position(|s| *s == step).unwrap_or(0) as u8 + 1;
        Arc::new(move |fraction| sink(Progress { meeting_id, step, index, total, fraction }))
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
        let mixed = mix(&mic, &system);
        let mix_path = dir.join(MIX_WAV);
        if !mix_path.exists() {
            write_wav(&mix_path, &mixed)?;
        }
        Ok(Tracks { mixed, mic, system })
    }

    fn step(&self, id: Uuid, step: Step, t: &Tracks, settings: &Settings, progress: &ProgressFn) -> Result<()> {
        let output = match step {
            Step::Transcribe => {
                let lang = Some(settings.transcription_language.as_str()).filter(|l| *l != "auto");
                let transcript = self.transcriber.transcribe_with_progress(&t.mixed, lang, progress)?;
                self.db().set_language(id, &transcript.language)?;
                serde_json::to_value(transcript)?
            }
            Step::Diarize => serde_json::to_value(self.diarizer.diarize_with_progress(&t.mixed, progress)?)?,
            Step::Identify => {
                let db = self.db();
                let transcript: Transcript = load(&db, id, Step::Transcribe)?;
                let turns: Vec<SpeakerTurn> = load(&db, id, Step::Diarize)?;
                let labels = label_turns(&turns, &t.mic, &t.system);
                let mut segments = assign_segments(&transcript, &turns, &labels);
                for s in &mut segments {
                    s.text = vocabulary::correct(&s.text, &settings.vocabulary);
                }
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
        asr: Arc<dyn Transcriber>,
        llm: Option<Arc<dyn Structurer>>,
    ) -> Pipeline {
        Pipeline {
            store: store.clone(),
            transcriber: asr,
            diarizer: Arc::new(FakeDiarizer),
            structurer: llm,
            data_dir: data_dir.into(),
            progress: None,
        }
    }

    type Reports = Arc<Mutex<Vec<Progress>>>;

    fn recording(p: &mut Pipeline) -> Reports {
        let seen: Reports = Arc::default();
        let sink = seen.clone();
        p.progress = Some(Arc::new(move |r| sink.lock().unwrap().push(r)));
        seen
    }

    #[test]
    fn reports_progress_per_step_in_order() {
        let (store, dir, id) = setup(&[]);
        let mut p = pipeline(&store, &dir, Arc::new(FakeTranscriber::default()), None);
        let seen = recording(&mut p);
        p.run(id).unwrap();
        let seen = seen.lock().unwrap();
        let got: Vec<(Step, u8, u8, f32)> = seen.iter().map(|r| (r.step, r.index, r.total, r.fraction)).collect();
        let mut want = Vec::new();
        for (i, step) in Step::ALL[..4].iter().enumerate() {
            want.push((*step, i as u8 + 1, 4, 0.0));
            want.push((*step, i as u8 + 1, 4, 1.0));
        }
        assert_eq!(got, want);
        assert!(seen.iter().all(|r| r.meeting_id == id));
        std::fs::remove_dir_all(dir).unwrap();
    }

    #[test]
    fn resumed_run_reports_skipped_steps_as_done() {
        let (store, dir, id) = setup(&[]);
        assert!(pipeline(&store, &dir, Arc::new(QuotaTranscriber), None).run(id).is_err());
        let transcript = FakeTranscriber::default().transcribe(&[], None).unwrap();
        lock(&store).save_step(id, Step::Transcribe, &serde_json::to_value(transcript).unwrap()).unwrap();
        let mut p = pipeline(&store, &dir, Arc::new(QuotaTranscriber), None);
        let seen = recording(&mut p);
        p.run(id).unwrap();
        let seen = seen.lock().unwrap();
        assert_eq!((seen[0].step, seen[0].index, seen[0].fraction), (Step::Transcribe, 1, 1.0));
        assert!(seen.iter().all(|r| r.step != Step::Transcribe || r.fraction == 1.0));
        assert_eq!(seen.last().map(|r| (r.step, r.fraction)), Some((Step::Review, 1.0)));
        std::fs::remove_dir_all(dir).unwrap();
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
    fn segments_get_vocabulary_spelling() {
        let (store, dir, id) = setup(&[]);
        {
            let db = lock(&store);
            let mut settings = db.settings().unwrap();
            settings.vocabulary = vec!["Salüt".into()];
            db.save_settings(&settings).unwrap();
        }
        pipeline(&store, &dir, Arc::new(FakeTranscriber::default()), None)
            .run(id)
            .unwrap();
        let texts: Vec<_> = lock(&store)
            .segments(id)
            .unwrap()
            .into_iter()
            .map(|s| s.text)
            .collect();
        assert_eq!(texts, vec!["bonjour", "Salüt"]);
        std::fs::remove_dir_all(dir).unwrap();
    }

    #[test]
    fn unreachable_llm_does_not_block_review_and_is_retried() {
        let (store, dir, id) = setup(&[]);
        let asr = Arc::new(FakeTranscriber::default());
        let p = pipeline(
            &store,
            &dir,
            asr.clone(),
            Some(Arc::new(FlakyStructurer::default())),
        );
        p.run(id).unwrap();
        let m = lock(&store).get_meeting(id).unwrap();
        assert_eq!(m.state, MeetingState::NeedsReview);
        assert!(lock(&store).meeting_doc(id).unwrap().summary.is_none());

        // The next run only redoes the missing summary.
        p.run(id).unwrap();
        let db = lock(&store);
        assert_eq!(asr.0.load(Ordering::SeqCst), 1);
        assert_eq!(db.meeting_doc(id).unwrap().summary.unwrap().overview, "ok");
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

    struct QuotaTranscriber;
    impl Transcriber for QuotaTranscriber {
        fn name(&self) -> &str {
            "fake-cloud"
        }
        fn transcribe(&self, _: &[f32], _: Option<&str>) -> Result<Transcript> {
            Err(Coded("mistral_quota").into())
        }
    }

    #[test]
    fn provider_error_code_is_stored() {
        let (store, dir, id) = setup(&[]);
        assert!(pipeline(&store, &dir, Arc::new(QuotaTranscriber), None)
            .run(id)
            .is_err());
        let m = lock(&store).get_meeting(id).unwrap();
        assert_eq!(
            (m.state, m.error.as_deref()),
            (MeetingState::Failed, Some("mistral_quota"))
        );
        std::fs::remove_dir_all(dir).unwrap();
    }

    fn llm(base_url: String) -> Option<Arc<dyn Structurer>> {
        Some(Arc::new(OpenAiStructurer {
            base_url,
            model: "m".into(),
            api_key: None,
            vocabulary: Vec::new(),
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
