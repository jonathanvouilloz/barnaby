//! WAV I/O and simple signal helpers. All output is 16 kHz mono f32.

use std::fs::OpenOptions;
use std::io::{Read, Seek, SeekFrom, Write};
use std::path::Path;

use anyhow::{Context, Result};
use hound::SampleFormat;

use crate::model::SAMPLE_RATE;

/// Reads any PCM WAV (i8/16/24/32 or f32, any rate and channel count) as 16 kHz mono.
pub fn read_wav(path: &Path) -> Result<Vec<f32>> {
    let mut reader = hound::WavReader::open(path).context("wav_open")?;
    let spec = reader.spec();
    let interleaved: Vec<f32> = match spec.sample_format {
        SampleFormat::Float => reader.samples::<f32>().collect::<Result<_, _>>()?,
        SampleFormat::Int => {
            let scale = (1i64 << (spec.bits_per_sample - 1)) as f32;
            reader
                .samples::<i32>()
                .map(|s| s.map(|v| v as f32 / scale))
                .collect::<Result<_, _>>()?
        }
    };
    let channels = spec.channels.max(1) as usize;
    let mono: Vec<f32> = interleaved
        .chunks_exact(channels)
        .map(|f| f.iter().sum::<f32>() / channels as f32)
        .collect();
    Ok(resample(&mono, spec.sample_rate, SAMPLE_RATE))
}

/// Linear interpolation; good enough for speech going into whisper.
/// Writes 16 kHz mono i16 (used for the review player's mixed track).
pub fn write_wav(path: &Path, samples: &[f32]) -> Result<()> {
    let spec = hound::WavSpec {
        channels: 1,
        sample_rate: SAMPLE_RATE,
        bits_per_sample: 16,
        sample_format: SampleFormat::Int,
    };
    let mut w = hound::WavWriter::create(path, spec)?;
    for s in samples {
        w.write_sample((s.clamp(-1.0, 1.0) * i16::MAX as f32) as i16)?;
    }
    w.finalize()?;
    Ok(())
}

pub fn resample(samples: &[f32], from: u32, to: u32) -> Vec<f32> {
    if from == to || samples.is_empty() {
        return samples.to_vec();
    }
    let ratio = from as f64 / to as f64;
    let len = (samples.len() as f64 / ratio) as usize;
    (0..len)
        .map(|i| {
            let pos = i as f64 * ratio;
            let j = pos as usize;
            let frac = (pos - j as f64) as f32;
            let a = samples[j];
            let b = *samples.get(j + 1).unwrap_or(&a);
            a + (b - a) * frac
        })
        .collect()
}

/// Rewrites the RIFF and `data` sizes from the file length, for a WAV whose
/// header was never finalized (crash during capture). Returns whether it changed.
pub fn repair_wav(path: &Path) -> Result<bool> {
    let mut f = OpenOptions::new()
        .read(true)
        .write(true)
        .open(path)
        .context("wav_open")?;
    let len = f.metadata()?.len();
    let mut header = [0u8; 12];
    f.read_exact(&mut header).context("wav_header")?;
    anyhow::ensure!(
        &header[0..4] == b"RIFF" && &header[8..12] == b"WAVE",
        "wav_header"
    );

    let mut block_align = 1u64;
    let mut pos = 12u64;
    loop {
        let mut chunk = [0u8; 8];
        f.seek(SeekFrom::Start(pos))?;
        f.read_exact(&mut chunk).context("wav_no_data_chunk")?;
        let size = u32::from_le_bytes([chunk[4], chunk[5], chunk[6], chunk[7]]) as u64;
        let body = pos + 8;
        match &chunk[0..4] {
            b"fmt " => {
                let mut fmt = [0u8; 14];
                f.read_exact(&mut fmt)?;
                block_align = u16::from_le_bytes([fmt[12], fmt[13]]).max(1) as u64;
            }
            // `data` is the last chunk our recorder writes; everything after its start is audio.
            b"data" => {
                let data_len = len.saturating_sub(body) / block_align * block_align;
                let riff_len = len - 8;
                let changed = size != data_len
                    || u32::from_le_bytes([header[4], header[5], header[6], header[7]]) as u64
                        != riff_len;
                if changed {
                    f.seek(SeekFrom::Start(4))?;
                    f.write_all(&(riff_len.min(u32::MAX as u64) as u32).to_le_bytes())?;
                    f.seek(SeekFrom::Start(pos + 4))?;
                    f.write_all(&(data_len.min(u32::MAX as u64) as u32).to_le_bytes())?;
                    f.sync_all()?;
                }
                return Ok(changed);
            }
            _ => {}
        }
        pos = body + size + (size & 1);
    }
}

/// Sums two tracks (padding the shorter with silence), clamped to [-1, 1].
pub fn mix(a: &[f32], b: &[f32]) -> Vec<f32> {
    (0..a.len().max(b.len()))
        .map(|i| (a.get(i).unwrap_or(&0.0) + b.get(i).unwrap_or(&0.0)).clamp(-1.0, 1.0))
        .collect()
}

pub fn rms(samples: &[f32]) -> f32 {
    if samples.is_empty() {
        return 0.0;
    }
    (samples.iter().map(|s| s * s).sum::<f32>() / samples.len() as f32).sqrt()
}

/// RMS of consecutive `window_s`-second windows (the last one may be shorter).
pub fn rms_windows(samples: &[f32], window_s: f64) -> Vec<f32> {
    let n = ((window_s * SAMPLE_RATE as f64) as usize).max(1);
    samples.chunks(n).map(rms).collect()
}

/// Samples between `start` and `end` seconds, clamped to the track.
pub fn slice(samples: &[f32], start: f64, end: f64) -> &[f32] {
    let idx = |t: f64| ((t.max(0.0) * SAMPLE_RATE as f64) as usize).min(samples.len());
    let (a, b) = (idx(start), idx(end));
    &samples[a..b.max(a)]
}

#[cfg(test)]
pub(crate) fn write_test_wav(path: &Path, samples: &[f32]) {
    let spec = hound::WavSpec {
        channels: 1,
        sample_rate: SAMPLE_RATE,
        bits_per_sample: 16,
        sample_format: SampleFormat::Int,
    };
    let mut w = hound::WavWriter::create(path, spec).unwrap();
    for s in samples {
        w.write_sample((s * i16::MAX as f32) as i16).unwrap();
    }
    w.finalize().unwrap();
}

#[cfg(test)]
mod tests {
    use super::*;

    fn tmp(name: &str) -> std::path::PathBuf {
        std::env::temp_dir().join(format!("barnaby-{}-{name}", uuid::Uuid::new_v4()))
    }

    #[test]
    fn reads_stereo_f32_48k_as_16k_mono() {
        let path = tmp("stereo.wav");
        let spec = hound::WavSpec {
            channels: 2,
            sample_rate: 48_000,
            bits_per_sample: 32,
            sample_format: SampleFormat::Float,
        };
        let mut w = hound::WavWriter::create(&path, spec).unwrap();
        for _ in 0..48_000 {
            w.write_sample(0.5f32).unwrap();
            w.write_sample(0.1f32).unwrap();
        }
        w.finalize().unwrap();
        let out = read_wav(&path).unwrap();
        assert_eq!(out.len(), 16_000);
        assert!(out.iter().all(|s| (s - 0.3).abs() < 1e-6));
        std::fs::remove_file(path).unwrap();
    }

    #[test]
    fn repairs_unfinalized_header() {
        let path = tmp("crash.wav");
        write_test_wav(&path, &vec![0.25; 16_000]);
        let mut bytes = std::fs::read(&path).unwrap();
        // Simulate a crash: header sizes from an early flush, plus a torn last sample.
        bytes[4..8].copy_from_slice(&36u32.to_le_bytes());
        bytes[40..44].copy_from_slice(&0u32.to_le_bytes());
        bytes.push(0);
        std::fs::write(&path, &bytes).unwrap();

        assert!(repair_wav(&path).unwrap());
        assert_eq!(read_wav(&path).unwrap().len(), 16_000);
        assert!(!repair_wav(&path).unwrap());
        std::fs::remove_file(path).unwrap();
    }

    #[test]
    fn mix_and_rms() {
        assert_eq!(mix(&[0.5, 0.9], &[0.2]), vec![0.7, 0.9]);
        assert_eq!(mix(&[0.8], &[0.8]), vec![1.0]);
        let w = rms_windows(&[0.5; 16_000 * 2], 0.5);
        assert_eq!(w.len(), 4);
        assert!((w[0] - 0.5).abs() < 1e-6);
        assert_eq!(slice(&[0.0; 16_000], 0.5, 2.0).len(), 8_000);
        assert!(slice(&[0.0; 10], 5.0, 1.0).is_empty());
    }
}
