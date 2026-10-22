//! Detection and resolution of review items ("zones d'ombre", spec section 13).

use anyhow::Result;
use uuid::Uuid;

use crate::audio::{rms_windows, slice};
use crate::model::{Resolution, ReviewItem, ReviewKind, Segment};
use crate::store::Store;

/// RMS above which a track counts as active (about -40 dBFS).
pub const ENERGY_FLOOR: f32 = 0.01;
const WINDOW_S: f64 = 0.1;
const MIN_OVERLAP_S: f64 = 1.0;
const INAUDIBLE_MARKERS: &[&str] = &[
    "[blank_audio]",
    "(inaudible)",
    "[inaudible]",
    "[silence]",
    "(silence)",
];

pub fn question_key(kind: ReviewKind) -> &'static str {
    match kind {
        ReviewKind::LowConfidence => "review.q.low_confidence",
        ReviewKind::Overlap => "review.q.overlap",
        ReviewKind::UnattributedSpeaker => "review.q.unattributed_speaker",
        ReviewKind::UncertainTerm => "review.q.uncertain_term",
        ReviewKind::Inaudible => "review.q.inaudible",
    }
}

/// One item per (segment, problem). `mic`/`system`: 16 kHz tracks, used for overlap.
pub fn detect(
    meeting_id: Uuid,
    segments: &[Segment],
    mic: &[f32],
    system: &[f32],
    threshold: f32,
) -> Vec<ReviewItem> {
    let mut items = Vec::new();
    for s in segments {
        let mut kinds = Vec::new();
        if is_inaudible(&s.text) {
            kinds.push(ReviewKind::Inaudible);
        } else if s.confidence < threshold {
            kinds.push(ReviewKind::LowConfidence);
        }
        if has_overlap(slice(mic, s.start, s.end), slice(system, s.start, s.end)) {
            kinds.push(ReviewKind::Overlap);
        }
        if s.speaker_id.is_none() {
            kinds.push(ReviewKind::UnattributedSpeaker);
        }
        items.extend(kinds.into_iter().map(|kind| ReviewItem {
            id: Uuid::new_v4().to_string(),
            meeting_id,
            segment_id: Some(s.id.clone()),
            kind,
            start: s.start,
            end: s.end,
            proposed_text: s.text.clone(),
            alternatives: Vec::new(),
            question: question_key(kind).into(),
            resolution: None,
        }));
    }
    items
}

fn is_inaudible(text: &str) -> bool {
    let t = text.trim().to_lowercase();
    t.is_empty() || INAUDIBLE_MARKERS.contains(&t.as_str())
}

/// Both tracks above the floor for more than a second in a row.
fn has_overlap(mic: &[f32], system: &[f32]) -> bool {
    let (m, s) = (rms_windows(mic, WINDOW_S), rms_windows(system, WINDOW_S));
    let mut run = 0usize;
    for (a, b) in m.iter().zip(&s) {
        run = if *a > ENERGY_FLOOR && *b > ENERGY_FLOOR {
            run + 1
        } else {
            0
        };
        if run as f64 * WINDOW_S > MIN_OVERLAP_S {
            return true;
        }
    }
    false
}

/// Applies the user's answer to the item and its segment. Every action except
/// `Ignore` marks the segment reviewed.
pub fn apply_resolution(store: &Store, item_id: &str, resolution: Resolution) -> Result<()> {
    let item = store.get_review_item(item_id)?;
    if let Some(seg) = &item.segment_id {
        let m = item.meeting_id;
        match &resolution {
            Resolution::Validate => store.mark_segment_reviewed(m, seg)?,
            Resolution::Correct { text } => store.update_segment_text(m, seg, text)?,
            Resolution::Reassign { speaker_id } => {
                store.update_segment_speaker(m, seg, speaker_id)?
            }
            Resolution::MarkInaudible => store.update_segment_text(m, seg, "[inaudible]")?,
            Resolution::Ignore => {}
        }
    }
    store.set_review_resolution(item_id, &resolution)
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::model::ProcessingMode;

    fn seg(id: &str, start: f64, text: &str, confidence: f32, speaker: Option<&str>) -> Segment {
        Segment {
            id: id.into(),
            start,
            end: start + 2.0,
            speaker_id: speaker.map(Into::into),
            text: text.into(),
            confidence,
            reviewed: false,
        }
    }

    fn kinds(items: &[ReviewItem], seg: &str) -> Vec<ReviewKind> {
        items
            .iter()
            .filter(|i| i.segment_id.as_deref() == Some(seg))
            .map(|i| i.kind)
            .collect()
    }

    #[test]
    fn detects_each_kind() {
        let mic = vec![0.1f32; 16_000 * 10];
        let mut system = vec![0.0f32; 16_000 * 10];
        // System active over 6.0..7.5 s: overlap of 1.5 s with segment s4 (6..8).
        system[16_000 * 6..16_000 * 15 / 2].fill(0.1);
        let segments = vec![
            seg("s1", 0.0, "fine", 0.9, Some("me")),
            seg("s2", 2.0, "unsure", 0.3, Some("me")),
            seg("s3", 4.0, " [BLANK_AUDIO] ", 0.1, None),
            seg("s4", 6.0, "both", 0.9, Some("me")),
        ];
        let items = detect(Uuid::nil(), &segments, &mic, &system, 0.6);
        assert!(kinds(&items, "s1").is_empty());
        assert_eq!(kinds(&items, "s2"), vec![ReviewKind::LowConfidence]);
        assert_eq!(
            kinds(&items, "s3"),
            vec![ReviewKind::Inaudible, ReviewKind::UnattributedSpeaker]
        );
        assert_eq!(kinds(&items, "s4"), vec![ReviewKind::Overlap]);
        assert!(items.iter().all(|i| i.question == question_key(i.kind)));
    }

    #[test]
    fn short_overlap_is_ignored() {
        let mic = vec![0.1f32; 16_000 * 2];
        let mut system = vec![0.0f32; 16_000 * 2];
        system[..16_000 * 8 / 10].fill(0.1);
        assert!(detect(
            Uuid::nil(),
            &[seg("s", 0.0, "x", 0.9, Some("me"))],
            &mic,
            &system,
            0.6
        )
        .is_empty());
    }

    #[test]
    fn resolutions_update_segments_and_counters() {
        let store = Store::open_in_memory().unwrap();
        let m = store
            .create_meeting("t", &[], ProcessingMode::Local)
            .unwrap()
            .id;
        let segments = vec![
            seg("s1", 0.0, "a", 0.1, Some("me")),
            seg("s2", 2.0, "b", 0.1, None),
            seg("s3", 4.0, "c", 0.1, Some("me")),
        ];
        store.save_segments(m, &segments).unwrap();
        let items = detect(m, &segments, &[], &[], 0.6);
        store.set_review_items(m, &items).unwrap();
        let meeting = store.get_meeting(m).unwrap();
        assert_eq!(
            (meeting.review_items_total, meeting.review_items_resolved),
            (4, 0)
        );

        let id_of = |seg: &str, kind| {
            items
                .iter()
                .find(|i| i.segment_id.as_deref() == Some(seg) && i.kind == kind)
                .unwrap()
                .id
                .clone()
        };
        apply_resolution(
            &store,
            &id_of("s1", ReviewKind::LowConfidence),
            Resolution::Correct {
                text: "fixed".into(),
            },
        )
        .unwrap();
        apply_resolution(
            &store,
            &id_of("s2", ReviewKind::UnattributedSpeaker),
            Resolution::Reassign {
                speaker_id: "p1".into(),
            },
        )
        .unwrap();
        apply_resolution(
            &store,
            &id_of("s3", ReviewKind::LowConfidence),
            Resolution::Ignore,
        )
        .unwrap();
        // Resolving twice must not double-count.
        apply_resolution(
            &store,
            &id_of("s3", ReviewKind::LowConfidence),
            Resolution::Validate,
        )
        .unwrap();

        let segs = store.segments(m).unwrap();
        assert_eq!((segs[0].text.as_str(), segs[0].reviewed), ("fixed", true));
        assert_eq!(
            (segs[1].speaker_id.as_deref(), segs[1].reviewed),
            (Some("p1"), true)
        );
        assert!(segs[2].reviewed);
        let meeting = store.get_meeting(m).unwrap();
        assert_eq!(
            (meeting.review_items_total, meeting.review_items_resolved),
            (4, 3)
        );
        let doc = store.meeting_doc(m).unwrap();
        assert_eq!(
            (
                doc.processing.review_items_total,
                doc.processing.review_items_resolved
            ),
            (4, 3)
        );
        assert!(apply_resolution(&store, "missing", Resolution::Ignore).is_err());
    }
}
