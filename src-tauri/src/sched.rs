//! Dictation pre-empts meeting transcription (architecture invariant 4): meetings are
//! transcribed in ~30 s blocks and a waiting dictation takes the model between two blocks.

use std::sync::{Arc, Condvar, Mutex};

use barnaby_core::{audio::rms_windows, RawSegment, Transcriber, Transcript, SAMPLE_RATE};

#[derive(Default)]
pub struct Gate {
    state: Mutex<GateState>,
    cv: Condvar,
}

#[derive(Default)]
struct GateState {
    busy: bool,
    urgent_waiting: usize,
}

pub struct GateGuard<'a>(&'a Gate);

impl Gate {
    /// Urgent callers (dictation) go before any non-urgent one waiting.
    pub fn acquire(&self, urgent: bool) -> GateGuard<'_> {
        let mut s = self.state.lock().unwrap_or_else(|e| e.into_inner());
        if urgent {
            s.urgent_waiting += 1;
        }
        while s.busy || (!urgent && s.urgent_waiting > 0) {
            s = self.cv.wait(s).unwrap_or_else(|e| e.into_inner());
        }
        if urgent {
            s.urgent_waiting -= 1;
        }
        s.busy = true;
        GateGuard(self)
    }
}

impl Drop for GateGuard<'_> {
    fn drop(&mut self) {
        self.0.state.lock().unwrap_or_else(|e| e.into_inner()).busy = false;
        self.0.cv.notify_all();
    }
}

const BLOCK_S: f64 = 30.0;
/// The cut is placed at the quietest 100 ms of the block's last seconds, to avoid splitting words.
const SEARCH_S: f64 = 5.0;

/// Wraps the meeting transcriber: block-wise, yielding the gate between blocks.
pub struct MeetingTranscriber {
    pub inner: Arc<dyn Transcriber>,
    pub gate: Arc<Gate>,
}

impl Transcriber for MeetingTranscriber {
    fn name(&self) -> &str {
        self.inner.name()
    }

    fn transcribe(&self, samples: &[f32], language: Option<&str>) -> anyhow::Result<Transcript> {
        let mut segments = Vec::new();
        let mut detected = None;
        for (start, end) in blocks(samples) {
            let offset = start as f64 / SAMPLE_RATE as f64;
            let part = {
                let _turn = self.gate.acquire(false);
                // Lock the language after the first block so blocks don't flip between languages.
                self.inner.transcribe(&samples[start..end], language.or(detected.as_deref()))?
            };
            detected.get_or_insert(part.language);
            segments.extend(part.segments.into_iter().map(|s| RawSegment { start: s.start + offset, end: s.end + offset, ..s }));
        }
        Ok(Transcript { language: detected.unwrap_or_else(|| language.unwrap_or("auto").to_string()), segments })
    }
}

fn blocks(samples: &[f32]) -> Vec<(usize, usize)> {
    let rate = SAMPLE_RATE as usize;
    let (block, search, win) = ((BLOCK_S as usize) * rate, (SEARCH_S as usize) * rate, rate / 10);
    let mut out = Vec::new();
    let mut start = 0;
    while samples.len() - start > block {
        let tail_start = start + block - search;
        let quietest = rms_windows(&samples[tail_start..start + block], 0.1)
            .iter()
            .enumerate()
            .min_by(|a, b| a.1.total_cmp(b.1))
            .map_or(search, |(i, _)| i * win + win / 2);
        let cut = tail_start + quietest;
        out.push((start, cut));
        start = cut;
    }
    out.push((start, samples.len()));
    out
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn blocks_cover_audio_and_cut_in_silence() {
        let rate = SAMPLE_RATE as usize;
        let mut samples = vec![0.5f32; 70 * rate];
        // Silence at 27.0–27.2 s: the first cut must land there.
        samples[27 * rate..27 * rate + rate / 5].fill(0.0);
        let b = blocks(&samples);
        assert_eq!(b.first().unwrap().0, 0);
        assert_eq!(b.last().unwrap().1, samples.len());
        assert!(b.windows(2).all(|w| w[0].1 == w[1].0));
        let cut_s = b[0].1 as f64 / rate as f64;
        assert!((27.0..27.2).contains(&cut_s), "cut at {cut_s}");
    }

    #[test]
    fn short_audio_is_one_block() {
        assert_eq!(blocks(&[0.0; 100]), vec![(0, 100)]);
        assert_eq!(blocks(&[]), vec![(0, 0)]);
    }

    #[test]
    fn urgent_waiter_goes_first() {
        let gate = Arc::new(Gate::default());
        let order = Arc::new(Mutex::new(Vec::new()));
        let held = gate.acquire(false);
        let spawn = |urgent: bool, tag: &'static str| {
            let (g, o) = (gate.clone(), order.clone());
            std::thread::spawn(move || {
                let _t = g.acquire(urgent);
                o.lock().unwrap().push(tag);
            })
        };
        let normal = spawn(false, "meeting");
        std::thread::sleep(std::time::Duration::from_millis(50));
        let urgent = spawn(true, "dictation");
        std::thread::sleep(std::time::Duration::from_millis(50));
        drop(held);
        normal.join().unwrap();
        urgent.join().unwrap();
        assert_eq!(*order.lock().unwrap(), vec!["dictation", "meeting"]);
    }
}
