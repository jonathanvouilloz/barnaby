//! Meeting capture (spec 9): `mic.wav` and `system.wav`, 16 kHz mono i16, written
//! as audio arrives so a crash loses at most the last flush interval.

use std::collections::{HashMap, HashSet, VecDeque};
use std::path::Path;
use std::sync::atomic::{AtomicBool, AtomicU32, AtomicU64, Ordering};
use std::sync::mpsc::{self, Receiver, RecvTimeoutError, Sender};
use std::sync::{Arc, Mutex};
use std::thread::{self, JoinHandle};
use std::time::{Duration, Instant};

use anyhow::Result;
use barnaby_core::SAMPLE_RATE;
use cpal::traits::{DeviceTrait, HostTrait};

use crate::dsp::{downmix, rms, to_i16, Resampler};
use crate::input;
use crate::outputs::{self, Source};

pub const SYSTEM_SILENT: &str = "capture.system_silent";
pub const NO_MIC: &str = "capture.no_mic";
pub const NO_SYSTEM: &str = "capture.no_system";
pub const PERMISSION_DENIED: &str = "capture.permission_denied";

const FLUSH_EVERY: Duration = Duration::from_secs(2);
const SILENCE_WARNING: Duration = Duration::from_secs(30);
/// About -60 dBFS.
const SILENCE_RMS: f32 = 1e-3;
/// WASAPI loopback delivers nothing while nothing plays, and a paused mic drops its
/// chunks: once a track lags the wall clock by this much, it is padded with silence
/// so both tracks stay aligned with the meeting timeline.
const GAP_TOLERANCE: u64 = SAMPLE_RATE as u64 / 2;
const GAP_MARGIN: u64 = SAMPLE_RATE as u64 / 10;
/// Outputs come and go mid-meeting (headset plugged in, Bluetooth hands-free profile).
const RESCAN_EVERY: Duration = Duration::from_secs(3);
const MIX_EVERY: Duration = Duration::from_millis(100);
/// The mix trails the wall clock so late chunks still land in place.
const MIX_DELAY: Duration = Duration::from_millis(200);
/// 1 s at 16 kHz: an output whose clock runs ahead loses its oldest samples past this.
const MAX_BACKLOG: usize = SAMPLE_RATE as usize;

fn wav_spec() -> hound::WavSpec {
    hound::WavSpec { channels: 1, sample_rate: SAMPLE_RATE, bits_per_sample: 16, sample_format: hound::SampleFormat::Int }
}

#[derive(Default)]
struct Track {
    /// f32 bits of the last chunk's RMS.
    level: AtomicU32,
    paused: AtomicBool,
    /// Milliseconds from start to the last chunk above the silence floor.
    last_sound_ms: AtomicU64,
}

struct Shared {
    start: Instant,
    stopping: AtomicBool,
    mic: Track,
    system: Track,
    warnings: Mutex<Vec<&'static str>>,
}

impl Shared {
    fn warn(&self, code: &'static str) {
        log::warn!("{code}");
        let mut w = self.warnings.lock().unwrap_or_else(|e| e.into_inner());
        if !w.contains(&code) {
            w.push(code);
        }
    }
}

pub struct Recorder {
    shared: Arc<Shared>,
    streams: Vec<cpal::Stream>,
    writers: Vec<JoinHandle<Result<()>>>,
}

impl Recorder {
    /// Starts both tracks in `dir`; `system.wav` mixes every output except `excluded`.
    /// A track that cannot start is reported through [`Recorder::warnings`] and left as
    /// a valid empty WAV; the other keeps recording.
    pub fn start(dir: &Path, excluded: &[String]) -> Result<Recorder> {
        std::fs::create_dir_all(dir)?;
        let shared = Arc::new(Shared {
            start: Instant::now(),
            stopping: AtomicBool::new(false),
            mic: Track::default(),
            system: Track::default(),
            warnings: Mutex::new(Vec::new()),
        });
        let mut rec = Recorder {
            shared: shared.clone(),
            streams: Vec::new(),
            writers: Vec::new(),
        };

        let mic_path = dir.join("mic.wav");
        let (tx, rx) = mpsc::channel();
        match open_mic(tx) {
            Ok(o) => {
                rec.writers.push(spawn_writer(&mic_path, rx, o.rate, o.channels, shared.clone(), |s| &s.mic)?);
                rec.streams.push(o.stream);
            }
            Err(e) => {
                shared.warn(if e.kind() == cpal::ErrorKind::PermissionDenied { PERMISSION_DENIED } else { NO_MIC });
                write_empty(&mic_path)?;
            }
        }

        let (tx, rx) = mpsc::channel();
        let writer = spawn_writer(&dir.join("system.wav"), rx, SAMPLE_RATE, 1, shared.clone(), |s| &s.system)?;
        let excluded = excluded.to_vec();
        rec.writers.push(thread::spawn(move || {
            mix_outputs(&tx, &excluded, &shared);
            drop(tx);
            writer.join().unwrap_or_else(|_| Err(anyhow::anyhow!("capture writer panicked")))
        }));
        Ok(rec)
    }

    /// RMS of the latest (mic, system) chunks, in [0, 1].
    pub fn levels(&self) -> (f32, f32) {
        let get = |t: &Track| f32::from_bits(t.level.load(Ordering::Relaxed));
        (get(&self.shared.mic), get(&self.shared.system))
    }

    /// Active warning codes (`capture.*`), to poll alongside [`Recorder::levels`].
    pub fn warnings(&self) -> Vec<&'static str> {
        let mut w = self.shared.warnings.lock().unwrap_or_else(|e| e.into_inner()).clone();
        let since_sound = self.shared.start.elapsed().saturating_sub(Duration::from_millis(self.shared.system.last_sound_ms.load(Ordering::Relaxed)));
        if !w.contains(&NO_SYSTEM) && since_sound > SILENCE_WARNING {
            w.push(SYSTEM_SILENT);
        }
        w
    }

    /// Spec 7.4: while paused the mic track records silence, keeping its timeline.
    pub fn pause_mic(&self, paused: bool) {
        self.shared.mic.paused.store(paused, Ordering::Relaxed);
    }

    /// Stops both tracks and finalizes their WAV headers.
    pub fn stop(mut self) -> Result<()> {
        self.shared.stopping.store(true, Ordering::Relaxed);
        // Dropping a stream drops its sender, which ends the writer loop; the system
        // mixer sees `stopping` and closes its outputs.
        self.streams.clear();
        let mut result = Ok(());
        for writer in self.writers.drain(..) {
            let r = writer.join().unwrap_or_else(|_| Err(anyhow::anyhow!("capture writer panicked")));
            if result.is_ok() {
                result = r;
            }
        }
        result
    }
}

fn write_empty(path: &Path) -> Result<()> {
    Ok(hound::WavWriter::create(path, wav_spec())?.finalize()?)
}

fn sender(tx: Sender<Vec<f32>>) -> impl FnMut(Vec<f32>) + Send + 'static {
    move |chunk| {
        let _ = tx.send(chunk);
    }
}

pub(crate) fn open_mic(tx: Sender<Vec<f32>>) -> Result<input::Opened, cpal::Error> {
    let device = cpal::default_host().default_input_device().ok_or(cpal::Error::new(cpal::ErrorKind::DeviceNotAvailable))?;
    let config = device.default_input_config()?;
    input::open(&device, config, sender(tx))
}

/// Sums the 16 kHz mono tracks of several outputs into one.
#[derive(Default)]
struct Mixer {
    queues: HashMap<String, VecDeque<f32>>,
}

impl Mixer {
    fn push(&mut self, id: &str, mono: &[f32]) {
        let q = self.queues.entry(id.to_owned()).or_default();
        q.extend(mono);
        if q.len() > MAX_BACKLOG {
            q.drain(..q.len() - MAX_BACKLOG);
        }
    }

    /// The next `n` mixed samples; an output with nothing queued adds silence.
    fn take(&mut self, n: usize) -> Vec<f32> {
        let mut out = vec![0.0; n];
        for q in self.queues.values_mut() {
            for (o, s) in out.iter_mut().zip(q.drain(..n.min(q.len()))) {
                *o += s;
            }
        }
        out.iter_mut().for_each(|s| *s = s.clamp(-1.0, 1.0));
        out
    }

    fn remove(&mut self, id: &str) {
        self.queues.remove(id);
    }

    fn backlog(&self) -> usize {
        self.queues.values().map(VecDeque::len).max().unwrap_or(0)
    }
}

/// Captures every output not in `excluded` until `stopping`, sending the mix to `out`
/// in step with the wall clock.
fn mix_outputs(out: &Sender<Vec<f32>>, excluded: &[String], shared: &Shared) {
    let (tx, rx) = mpsc::channel::<(String, Vec<f32>)>();
    let mut sources: HashMap<String, (Source, Resampler)> = HashMap::new();
    let mut failed = HashSet::new();
    let mut mixer = Mixer::default();
    let drain = |sources: &mut HashMap<String, (Source, Resampler)>, mixer: &mut Mixer| {
        for (id, chunk) in rx.try_iter() {
            if let Some((src, resampler)) = sources.get_mut(&id) {
                mixer.push(&id, &resampler.process(&downmix(&chunk, src.channels)));
            }
        }
    };
    let mut emitted = 0u64;
    let mut last_scan: Option<Instant> = None;
    while !shared.stopping.load(Ordering::Relaxed) {
        if last_scan.is_none_or(|t| t.elapsed() >= RESCAN_EVERY) {
            rescan(&mut sources, &mut failed, &mut mixer, excluded, &tx);
            if last_scan.is_none() && sources.is_empty() {
                shared.warn(NO_SYSTEM);
            }
            last_scan = Some(Instant::now());
        }
        thread::sleep(MIX_EVERY);
        drain(&mut sources, &mut mixer);
        let target = (shared.start.elapsed().saturating_sub(MIX_DELAY).as_secs_f64() * f64::from(SAMPLE_RATE)) as u64;
        if target > emitted {
            if out.send(mixer.take((target - emitted) as usize)).is_err() {
                return;
            }
            emitted = target;
        }
    }
    drain(&mut sources, &mut mixer);
    drop(sources);
    let _ = out.send(mixer.take(mixer.backlog()));
}

/// Opens new outputs and closes the ones that disappeared or died.
fn rescan(
    sources: &mut HashMap<String, (Source, Resampler)>,
    failed: &mut HashSet<String>,
    mixer: &mut Mixer,
    excluded: &[String],
    tx: &Sender<(String, Vec<f32>)>,
) {
    let listed = outputs::list();
    sources.retain(|id, (src, _)| {
        let keep = src.alive() && listed.iter().any(|o| &o.id == id);
        if !keep {
            log::info!("output closed: {id}");
            mixer.remove(id);
        }
        keep
    });
    failed.retain(|id| listed.iter().any(|o| &o.id == id));
    for o in listed {
        if excluded.contains(&o.id) || sources.contains_key(&o.id) || failed.contains(&o.id) {
            continue;
        }
        let (tx, id) = (tx.clone(), o.id.clone());
        match outputs::open(&o.id, move |chunk| {
            let _ = tx.send((id.clone(), chunk));
        }) {
            Ok(src) => {
                log::info!("output captured: {}", o.id);
                let resampler = Resampler::new(src.rate);
                sources.insert(o.id, (src, resampler));
            }
            Err(e) => {
                // Logged once while the output stays listed.
                log::warn!("output {} capture failed: {e}", o.id);
                failed.insert(o.id);
            }
        }
    }
}

fn spawn_writer(
    path: &Path,
    rx: Receiver<Vec<f32>>,
    rate: u32,
    channels: u16,
    shared: Arc<Shared>,
    track: fn(&Shared) -> &Track,
) -> Result<JoinHandle<Result<()>>> {
    let wav = hound::WavWriter::create(path, wav_spec())?;
    Ok(thread::spawn(move || write_track(wav, rx, rate, channels, shared.start, track(&shared))))
}

fn write_track<W: std::io::Write + std::io::Seek>(
    mut wav: hound::WavWriter<W>,
    rx: Receiver<Vec<f32>>,
    rate: u32,
    channels: u16,
    start: Instant,
    track: &Track,
) -> Result<()> {
    let mut resampler = Resampler::new(rate);
    let mut written: u64 = 0;
    let mut last_flush = Instant::now();
    loop {
        match rx.recv_timeout(Duration::from_millis(100)) {
            Ok(chunk) if !track.paused.load(Ordering::Relaxed) => {
                let mono = resampler.process(&downmix(&chunk, channels));
                let level = rms(&mono);
                track.level.store(level.to_bits(), Ordering::Relaxed);
                if level > SILENCE_RMS {
                    track.last_sound_ms.store(start.elapsed().as_millis() as u64, Ordering::Relaxed);
                }
                for s in &mono {
                    wav.write_sample(to_i16(*s))?;
                }
                written += mono.len() as u64;
            }
            Ok(_) | Err(RecvTimeoutError::Timeout) => track.level.store(0, Ordering::Relaxed),
            Err(RecvTimeoutError::Disconnected) => break,
        }
        let expected = (start.elapsed().as_secs_f64() * f64::from(SAMPLE_RATE)) as u64;
        if expected > written + GAP_TOLERANCE {
            let pad = expected - written - GAP_MARGIN;
            for _ in 0..pad {
                wav.write_sample(0i16)?;
            }
            written += pad;
        }
        if last_flush.elapsed() >= FLUSH_EVERY {
            wav.flush()?;
            last_flush = Instant::now();
        }
    }
    Ok(wav.finalize()?)
}

#[cfg(test)]
mod tests {
    use super::*;

    fn read(path: &Path) -> Vec<i16> {
        hound::WavReader::open(path).unwrap().into_samples::<i16>().map(Result::unwrap).collect()
    }

    #[test]
    fn header_is_flushed_while_recording() {
        let dir = std::env::temp_dir().join(format!("barnaby-capture-{}", std::process::id()));
        std::fs::create_dir_all(&dir).unwrap();
        let path = dir.join("mic.wav");
        let (tx, rx) = mpsc::channel();
        let shared = Arc::new(Shared {
            start: Instant::now(),
            stopping: AtomicBool::new(false),
            mic: Track::default(),
            system: Track::default(),
            warnings: Mutex::new(Vec::new()),
        });
        // 48 kHz stereo device: 0.1 s chunks.
        let writer = spawn_writer(&path, rx, 48_000, 2, shared.clone(), |s| &s.mic).unwrap();
        for _ in 0..10 {
            tx.send(vec![0.5; 9_600]).unwrap();
        }
        thread::sleep(FLUSH_EVERY + Duration::from_millis(300));

        let samples = read(&path);
        assert!(samples.len() >= 16_000, "{}", samples.len());
        assert!(samples[..15_000].iter().all(|&s| s == to_i16(0.5)));
        assert!(shared.mic.level.load(Ordering::Relaxed) == 0, "level decays without audio");

        drop(tx);
        writer.join().unwrap().unwrap();
        // The gap after the last chunk was padded to the wall clock.
        assert!(read(&path).len() >= 16_000 + 8_000);
        std::fs::remove_dir_all(dir).unwrap();
    }

    #[test]
    fn mixer_sums_outputs_and_fills_gaps_with_silence() {
        let mut m = Mixer::default();
        assert_eq!(m.take(3), [0.0; 3], "no output yet");
        m.push("a", &[0.1, 0.2, 0.3]);
        m.push("b", &[0.1]);
        assert_eq!(m.take(3), [0.2, 0.2, 0.3]);
        // An output that starts late joins at the current position.
        m.push("c", &[0.5]);
        assert_eq!(m.take(2), [0.5, 0.0]);
    }

    #[test]
    fn mixer_clips_and_bounds_backlog() {
        let mut m = Mixer::default();
        m.push("a", &[0.8]);
        m.push("b", &[0.8]);
        assert_eq!(m.take(1), [1.0]);
        let backlog: Vec<f32> = (0..MAX_BACKLOG + 10).map(|i| i as f32).collect();
        m.push("a", &backlog);
        let out = m.take(MAX_BACKLOG + 10);
        assert_eq!(out[MAX_BACKLOG..], [0.0; 10], "oldest samples were dropped");
        m.remove("a");
        assert!(!m.queues.contains_key("a"));
    }

    #[test]
    fn mix_follows_the_wall_clock() {
        let shared = Arc::new(Shared {
            start: Instant::now(),
            stopping: AtomicBool::new(false),
            mic: Track::default(),
            system: Track::default(),
            warnings: Mutex::new(Vec::new()),
        });
        let (tx, rx) = mpsc::channel();
        let mixer = {
            let shared = shared.clone();
            // Real outputs excluded: the clock is under test, and opening the macOS tap waits on
            // a permission prompt nobody answers on CI.
            let excluded: Vec<String> = outputs::list().into_iter().map(|o| o.id).collect();
            thread::spawn(move || mix_outputs(&tx, &excluded, &shared))
        };
        thread::sleep(Duration::from_millis(700));
        shared.stopping.store(true, Ordering::Relaxed);
        mixer.join().unwrap();
        // Bound by the real elapsed time, not 0.7 s: slow CI runners oversleep.
        let max = (shared.start.elapsed().saturating_sub(MIX_DELAY).as_secs_f64() * f64::from(SAMPLE_RATE)) as usize;
        let total: usize = rx.try_iter().map(|c| c.len()).sum();
        // At least 0.5 s emitted on the clock (0.7 s minus the mix delay), never ahead of it.
        assert!((7_000..=max).contains(&total), "{total} not in 7000..={max}");
    }

    #[test]
    fn empty_track_is_a_valid_wav() {
        let path = std::env::temp_dir().join(format!("barnaby-empty-{}.wav", std::process::id()));
        write_empty(&path).unwrap();
        let reader = hound::WavReader::open(&path).unwrap();
        assert_eq!((reader.spec(), reader.len()), (wav_spec(), 0));
        std::fs::remove_file(path).unwrap();
    }
}
