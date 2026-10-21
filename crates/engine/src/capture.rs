//! Meeting capture (spec 9): `mic.wav` and `system.wav`, 16 kHz mono i16, written
//! as audio arrives so a crash loses at most the last flush interval.

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
    #[cfg(target_os = "linux")]
    parec: Option<std::process::Child>,
    writers: Vec<JoinHandle<Result<()>>>,
}

impl Recorder {
    /// Starts both tracks in `dir`. A track that cannot start is reported through
    /// [`Recorder::warnings`] and left as a valid empty WAV; the other keeps recording.
    pub fn start(dir: &Path) -> Result<Recorder> {
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
            #[cfg(target_os = "linux")]
            parec: None,
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

        let system_path = dir.join("system.wav");
        let (tx, rx) = mpsc::channel();
        #[cfg(target_os = "linux")]
        let started = start_parec(tx, shared.clone()).map(|child| {
            rec.parec = Some(child);
            (SAMPLE_RATE, 1)
        });
        #[cfg(not(target_os = "linux"))]
        let started = open_loopback(tx).map(|o| {
            let format = (o.rate, o.channels);
            rec.streams.push(o.stream);
            format
        });
        match started {
            Ok((rate, channels)) => rec.writers.push(spawn_writer(&system_path, rx, rate, channels, shared.clone(), |s| &s.system)?),
            Err(e) => {
                log::warn!("system capture failed: {e}");
                shared.warn(NO_SYSTEM);
                write_empty(&system_path)?;
            }
        }
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
        // Dropping a stream drops its sender, which ends the writer loop.
        self.streams.clear();
        #[cfg(target_os = "linux")]
        if let Some(mut child) = self.parec.take() {
            let _ = child.kill();
            let _ = child.wait();
        }
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

#[cfg(not(target_os = "linux"))]
fn open_loopback(tx: Sender<Vec<f32>>) -> Result<input::Opened, cpal::Error> {
    let device = cpal::default_host().default_output_device().ok_or(cpal::Error::new(cpal::ErrorKind::DeviceNotAvailable))?;
    let config = device.default_output_config()?;
    input::open(&device, config, sender(tx))
}

/// PulseAudio / PipeWire monitor of the default sink, already 16 kHz mono.
#[cfg(target_os = "linux")]
fn start_parec(tx: Sender<Vec<f32>>, shared: Arc<Shared>) -> std::io::Result<std::process::Child> {
    use std::io::Read;
    use std::process::{Command, Stdio};

    let mut child = Command::new("parec")
        .args(["--device=@DEFAULT_MONITOR@", "--format=s16le", "--rate=16000", "--channels=1", "--raw"])
        .stdout(Stdio::piped())
        .stderr(Stdio::null())
        .spawn()?;
    let Some(mut out) = child.stdout.take() else {
        return Err(std::io::Error::other("parec stdout not piped"));
    };
    thread::spawn(move || {
        let mut buf = [0u8; 3200]; // 100 ms
        let mut got_data = false;
        while out.read_exact(&mut buf).is_ok() {
            got_data = true;
            let chunk = buf.as_chunks::<2>().0.iter().map(|&b| f32::from(i16::from_le_bytes(b)) / 32768.0).collect();
            if tx.send(chunk).is_err() {
                break;
            }
        }
        if !got_data && !shared.stopping.load(Ordering::Relaxed) {
            shared.warn(NO_SYSTEM);
        }
    });
    Ok(child)
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
    fn empty_track_is_a_valid_wav() {
        let path = std::env::temp_dir().join(format!("barnaby-empty-{}.wav", std::process::id()));
        write_empty(&path).unwrap();
        let reader = hound::WavReader::open(&path).unwrap();
        assert_eq!((reader.spec(), reader.len()), (wav_spec(), 0));
        std::fs::remove_file(path).unwrap();
    }
}
