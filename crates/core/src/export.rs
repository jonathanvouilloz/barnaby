//! Meeting export: `<id>.json` (the `MeetingDoc`) and a readable `<id>.md`.

use std::fmt::Write as _;
use std::path::Path;

use anyhow::Result;

use crate::model::MeetingDoc;

pub fn write_folder(doc: &MeetingDoc, dir: &Path) -> Result<()> {
    std::fs::create_dir_all(dir)?;
    let id = doc.meeting.id;
    std::fs::write(
        dir.join(format!("{id}.json")),
        serde_json::to_string_pretty(doc)?,
    )?;
    std::fs::write(dir.join(format!("{id}.md")), markdown(doc))?;
    Ok(())
}

/// Display name of a speaker id: participant name, else the raw id.
pub fn speaker_name<'a>(doc: &'a MeetingDoc, speaker_id: Option<&'a str>) -> &'a str {
    match speaker_id {
        Some(id) => doc
            .meeting
            .participants
            .iter()
            .find(|p| p.id == id)
            .map_or(id, |p| p.name.as_str()),
        None => "?",
    }
}

pub fn markdown(doc: &MeetingDoc) -> String {
    let m = &doc.meeting;
    let mut md = format!(
        "# {}\n\n{}\n",
        m.title,
        m.started_at.format("%Y-%m-%d %H:%M UTC")
    );
    // `write!` into a String cannot fail.
    if !m.participants.is_empty() {
        md.push_str("\n## Participants\n\n");
        for p in &m.participants {
            let _ = writeln!(md, "- {}", p.name);
        }
    }
    if let Some(s) = &doc.summary {
        md.push_str("\n## Résumé\n\n");
        let _ = writeln!(md, "{}", s.overview);
        list(&mut md, "Décisions", s.decisions.iter().cloned());
        list(
            &mut md,
            "Actions",
            s.action_items.iter().map(|a| {
                let owner = a
                    .owner
                    .as_deref()
                    .map(|o| format!("{} : ", speaker_name(doc, Some(o))))
                    .unwrap_or_default();
                let due = a
                    .due
                    .as_deref()
                    .map(|d| format!(" ({d})"))
                    .unwrap_or_default();
                format!("{owner}{}{due}", a.task)
            }),
        );
        list(
            &mut md,
            "Questions ouvertes",
            s.open_questions.iter().cloned(),
        );
    }
    md.push_str("\n## Transcription\n\n");
    for s in &doc.segments {
        let _ = writeln!(
            md,
            "**{}** [{}] {}\n",
            speaker_name(doc, s.speaker_id.as_deref()),
            mmss(s.start),
            s.text.trim()
        );
    }
    if !doc.notes.is_empty() {
        md.push_str("## Notes\n\n");
        for n in &doc.notes {
            let _ = writeln!(md, "- [{}] {}", mmss(n.at), n.text);
        }
    }
    md
}

fn list(md: &mut String, title: &str, items: impl Iterator<Item = String>) {
    let items: Vec<_> = items.collect();
    if !items.is_empty() {
        let _ = writeln!(md, "\n### {title}\n");
        for i in items {
            let _ = writeln!(md, "- {i}");
        }
    }
}

fn mmss(t: f64) -> String {
    let s = t.max(0.0) as u64;
    format!("{:02}:{:02}", s / 60, s % 60)
}

#[cfg(test)]
pub(crate) mod tests {
    use super::*;
    use crate::model::*;
    use chrono::DateTime;

    pub(crate) fn sample_doc() -> MeetingDoc {
        MeetingDoc {
            schema_version: SCHEMA_VERSION.into(),
            meeting: MeetingInfo {
                id: uuid::Uuid::new_v4(),
                title: "Point hebdo".into(),
                started_at: DateTime::from_timestamp(1_700_000_000, 0).unwrap(),
                ended_at: None,
                language: Some("fr".into()),
                participants: vec![Participant {
                    id: "me".into(),
                    name: "Moi".into(),
                    email: None,
                    confirmed: true,
                }],
            },
            segments: vec![
                Segment {
                    id: "s1".into(),
                    start: 75.2,
                    end: 80.0,
                    speaker_id: Some("me".into()),
                    text: "Bonjour".into(),
                    confidence: 0.9,
                    reviewed: true,
                },
                Segment {
                    id: "s2".into(),
                    start: 81.0,
                    end: 82.0,
                    speaker_id: Some("speaker_a".into()),
                    text: "Salut".into(),
                    confidence: 0.9,
                    reviewed: true,
                },
            ],
            notes: vec![Note {
                id: "n1".into(),
                at: 3.0,
                text: "budget".into(),
                source: "manual".into(),
            }],
            summary: Some(Summary {
                overview: "Tour d'horizon".into(),
                decisions: vec!["Livrer vendredi".into()],
                action_items: vec![ActionItem {
                    owner: Some("me".into()),
                    task: "Écrire la doc".into(),
                    due: None,
                }],
                open_questions: vec![],
            }),
            processing: Processing {
                mode: ProcessingMode::Local,
                providers: Default::default(),
                review_items_total: 0,
                review_items_resolved: 0,
            },
        }
    }

    #[test]
    fn writes_json_and_markdown() {
        let doc = sample_doc();
        let dir = std::env::temp_dir().join(format!("barnaby-export-{}", uuid::Uuid::new_v4()));
        write_folder(&doc, &dir).unwrap();
        let id = doc.meeting.id;
        let json: MeetingDoc =
            serde_json::from_str(&std::fs::read_to_string(dir.join(format!("{id}.json"))).unwrap())
                .unwrap();
        assert_eq!(json, doc);
        let md = std::fs::read_to_string(dir.join(format!("{id}.md"))).unwrap();
        assert!(md.starts_with("# Point hebdo\n\n2023-11-14 22:13 UTC\n"));
        assert!(md.contains("**Moi** [01:15] Bonjour"));
        assert!(md.contains("**speaker_a** [01:21] Salut"));
        assert!(md.contains("- Moi : Écrire la doc"));
        assert!(md.contains("### Décisions\n\n- Livrer vendredi"));
        assert!(!md.contains("Questions ouvertes"));
        assert!(md.contains("- [00:03] budget"));
        std::fs::remove_dir_all(dir).unwrap();
    }
}
