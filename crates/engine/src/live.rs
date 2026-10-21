//! Live dictation: each speech segment is transcribed while the user keeps talking,
//! so only the last one is left to transcribe when they stop.

use std::sync::mpsc::{self, Sender};
use std::thread::{self, JoinHandle};

use anyhow::{anyhow, Result};
use barnaby_core::{RawSegment, Transcript, SAMPLE_RATE};

use crate::vad::{Segment, VadStream};

pub struct LiveDictation {
    /// None (no VAD model): the whole recording is one segment, sent at the end.
    vad: Option<VadStream>,
    raw: Vec<f32>,
    tx: Sender<Segment>,
    worker: JoinHandle<Result<Transcript>>,
}

impl LiveDictation {
    /// `transcribe` runs on a worker thread, once per segment, in order. The first error
    /// stops the session: [`LiveDictation::finish`] returns it.
    pub fn new(vad: Option<VadStream>, mut transcribe: impl FnMut(&[f32]) -> Result<Transcript> + Send + 'static) -> LiveDictation {
        let (tx, rx) = mpsc::channel::<Segment>();
        let worker = thread::spawn(move || {
            let mut out = Transcript { language: String::new(), segments: Vec::new() };
            for (start, samples) in rx {
                let part = transcribe(&samples)?;
                let offset = start as f64 / f64::from(SAMPLE_RATE);
                if out.language.is_empty() {
                    out.language = part.language;
                }
                out.segments.extend(part.segments.into_iter().map(|s| RawSegment { start: s.start + offset, end: s.end + offset, ..s }));
            }
            Ok(out)
        });
        LiveDictation { vad, raw: Vec::new(), tx, worker }
    }

    /// Adds 16 kHz mono samples; returns how many segments were queued for transcription.
    pub fn push(&mut self, samples: &[f32]) -> usize {
        match self.vad.as_mut() {
            Some(vad) => {
                let segments = vad.push(samples);
                self.send(segments)
            }
            None => {
                self.raw.extend_from_slice(samples);
                0
            }
        }
    }

    /// Transcribes what is left and returns the whole dictation.
    pub fn finish(mut self) -> Result<Transcript> {
        let tail = match self.vad.take() {
            Some(vad) => vad.finish(),
            None => vec![(0, std::mem::take(&mut self.raw))],
        };
        self.send(tail);
        drop(self.tx);
        self.worker.join().map_err(|_| anyhow!("dictation worker panicked"))?
    }

    fn send(&self, segments: Vec<Segment>) -> usize {
        let n = segments.len();
        for s in segments {
            // A closed channel means the worker already failed; finish() reports why.
            let _ = self.tx.send(s);
        }
        n
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::sync::atomic::{AtomicUsize, Ordering};
    use std::sync::Arc;
    use std::time::{Duration, Instant};

    use barnaby_core::Transcriber;

    use crate::parakeet::{tests as pk, ParakeetTranscriber};

    fn fake(samples: &[f32]) -> Result<Transcript> {
        Ok(Transcript { language: "fr".into(), segments: vec![RawSegment { start: 0.0, end: samples.len() as f64 / 16_000.0, text: format!("{}", samples.len()), confidence: 1.0 }] })
    }

    #[test]
    fn without_vad_the_recording_is_one_segment() {
        let mut live = LiveDictation::new(None, fake);
        assert_eq!(live.push(&[0.1; 8_000]) + live.push(&[0.1; 8_000]), 0);
        let t = live.finish().unwrap();
        assert_eq!((t.language.as_str(), t.segments.len(), t.segments[0].text.as_str()), ("fr", 1, "16000"));
    }

    #[test]
    fn first_error_is_reported() {
        let live = LiveDictation::new(None, |_| Err(anyhow!("model_missing")));
        assert_eq!(live.finish().unwrap_err().to_string(), "model_missing");
    }

    /// 60 s dictation: fr.wav repeated with pauses, pushed faster than real time. Segments are
    /// transcribed as they finish; once the backlog is done, "stop" leaves only the tail.
    /// Uses `BARNABY_MODELS_DIR` for parakeet-v3 and silero-vad (downloaded if missing).
    #[test]
    #[ignore]
    fn stop_to_text_latency_for_a_minute_of_parakeet() {
        let dir = pk::model_dir();
        let vad = VadStream::new(&crate::models::tests::ensure("silero-vad")).unwrap();
        let t0 = Instant::now();
        let parakeet = Arc::new(ParakeetTranscriber::load(&dir).unwrap());
        eprintln!("parakeet load: {:.2} s", t0.elapsed().as_secs_f64());
        let speech = pk::french_sample(&dir);
        let pause = vec![0.0; 16_000 * 7 / 10];
        let mut audio = Vec::new();
        while audio.len() < 60 * 16_000 {
            audio.extend_from_slice(&speech);
            audio.extend_from_slice(&pause);
        }
        audio.truncate(audio.len() - pause.len());

        let done = Arc::new(AtomicUsize::new(0));
        let (p, d) = (parakeet.clone(), done.clone());
        let mut live = LiveDictation::new(Some(vad), move |s| {
            let t = p.transcribe(s, Some("fr"));
            d.fetch_add(1, Ordering::SeqCst);
            t
        });
        let mut queued = 0;
        for chunk in audio.chunks(800) {
            queued += live.push(chunk);
        }
        while done.load(Ordering::SeqCst) < queued {
            std::thread::sleep(Duration::from_millis(10));
        }
        let stop = Instant::now();
        let t = live.finish().unwrap();
        let latency = stop.elapsed();
        let text = t.segments.iter().map(|s| s.text.as_str()).collect::<Vec<_>>().join(" ");
        eprintln!("{:.0} s of audio, {queued} segments while recording, stop -> text {:.2} s", audio.len() as f64 / 16_000.0, latency.as_secs_f64());
        assert!(queued >= 5, "segments are transcribed during the recording");
        assert!(text.split_whitespace().count() > 50, "{text}");
        assert!(t.segments.windows(2).all(|w| w[0].end <= w[1].start + 0.5));
    }
}
