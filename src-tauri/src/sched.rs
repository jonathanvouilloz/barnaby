//! Dictation pre-empts meeting transcription (architecture invariant 4): meetings are
//! transcribed in ~30 s blocks and a waiting dictation takes the model between two blocks.

use std::sync::{Arc, Condvar, Mutex};

use barnaby_core::{
    audio::{self, rms}, RawSegment, Transcriber, Transcript, SAMPLE_RATE};

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
/// Below this a block is treated as silence (about -50 dBFS).
const SPEECH_RMS: f32 = 3e-3;

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
        for (start, end) in audio::blocks(samples, BLOCK_S) {
            let offset = start as f64 / SAMPLE_RATE as f64;
            let block = &samples[start..end];
            let part = {
                let _turn = self.gate.acquire(false);
                self.inner.transcribe(block, language.or(detected.as_deref()))?
            };
            // Lock the language on the first block with speech, so blocks don't flip between
            // languages; whisper guesses at random on the silence that often opens a meeting.
            if !part.segments.is_empty() && rms(block) > SPEECH_RMS {
                detected.get_or_insert(part.language);
            }
            segments.extend(part.segments.into_iter().map(|s| RawSegment { start: s.start + offset, end: s.end + offset, ..s }));
        }
        Ok(Transcript { language: detected.unwrap_or_else(|| language.unwrap_or("auto").to_string()), segments })
    }
}

#[cfg(test)]
mod tests {
    use super::*;

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
