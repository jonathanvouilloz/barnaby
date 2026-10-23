//! Dictation start cue: Jon's short sound (Pixabay), played on the default output.

use std::{io::Cursor, thread, time::Duration};

use cpal::traits::{DeviceTrait, HostTrait, StreamTrait};
use cpal::{FromSample, SampleFormat, SizedSample};

/// 0.12 s, mono, 48 kHz, trimmed from the original MP3.
const START: &[u8] = include_bytes!("../assets/start.wav");
/// The file is played at its own level: Jon picked it by ear.
const VOLUME: f32 = 1.0;

/// How long the mic should ignore after the cue starts: the sound plus output latency.
pub const MIC_MUTE: Duration = Duration::from_millis(160);

/// Plays the start cue and returns once it is done. Fails silently (logged).
pub fn play_start() {
    if let Err(e) = try_play() {
        log::warn!("cue: {e:#}");
    }
}

fn try_play() -> anyhow::Result<()> {
    let (sound, rate) = decode(START)?;
    let device = cpal::default_host().default_output_device().ok_or_else(|| anyhow::anyhow!("no output device"))?;
    let config = device.default_output_config()?;
    let channels = usize::from(config.channels());
    let samples = resample(&sound, rate, config.sample_rate());
    let duration = Duration::from_secs_f32(samples.len() as f32 / config.sample_rate() as f32);
    let cfg = config.config();
    let stream = match config.sample_format() {
        SampleFormat::I16 => build::<i16>(&device, cfg, samples, channels),
        SampleFormat::I32 => build::<i32>(&device, cfg, samples, channels),
        SampleFormat::U16 => build::<u16>(&device, cfg, samples, channels),
        _ => build::<f32>(&device, cfg, samples, channels),
    }?;
    stream.play()?;
    thread::sleep(duration + Duration::from_millis(80));
    Ok(())
}

/// Mono samples in -1..1 and their rate.
fn decode(wav: &[u8]) -> anyhow::Result<(Vec<f32>, u32)> {
    let mut reader = hound::WavReader::new(Cursor::new(wav))?;
    let rate = reader.spec().sample_rate;
    let samples = reader.samples::<i16>().map(|s| s.map(|v| VOLUME * f32::from(v) / 32768.0)).collect::<Result<_, _>>()?;
    Ok((samples, rate))
}

/// Linear interpolation, enough for a cue a tenth of a second long.
fn resample(s: &[f32], from: u32, to: u32) -> Vec<f32> {
    if from == to || s.is_empty() {
        return s.to_vec();
    }
    let n = (s.len() as u64 * u64::from(to) / u64::from(from)) as usize;
    let step = from as f64 / to as f64;
    (0..n)
        .map(|i| {
            let x = i as f64 * step;
            let (j, frac) = (x as usize, (x.fract()) as f32);
            let a = s[j.min(s.len() - 1)];
            let b = s[(j + 1).min(s.len() - 1)];
            a + (b - a) * frac
        })
        .collect()
}

fn build<T>(device: &cpal::Device, cfg: cpal::StreamConfig, samples: Vec<f32>, channels: usize) -> Result<cpal::Stream, cpal::Error>
where
    T: SizedSample + FromSample<f32>,
{
    let mut it = samples.into_iter();
    device.build_output_stream::<T, _, _>(
        cfg,
        move |out: &mut [T], _: &cpal::OutputCallbackInfo| {
            for frame in out.chunks_mut(channels) {
                let s = T::from_sample(it.next().unwrap_or(0.0));
                frame.iter_mut().for_each(|o| *o = s);
            }
        },
        |e| log::warn!("cue stream error: {:?}", e.kind()),
        None,
    )
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn start_sound_decodes() {
        let (s, rate) = decode(START).unwrap();
        assert_eq!(rate, 48_000);
        assert!(!s.is_empty() && s.len() <= 48_000 / 4, "cue must stay short: {}", s.len());
        assert!(s.iter().all(|x| x.abs() <= 1.0));
        assert!(s.iter().any(|x| x.abs() > 0.05), "cue is silent");
    }

    #[test]
    fn resample_keeps_duration() {
        let s: Vec<f32> = (0..4800).map(|i| (i as f32 * 0.01).sin()).collect();
        let out = resample(&s, 48_000, 44_100);
        assert!(out.len().abs_diff(4800 * 44_100 / 48_000) <= 1);
        assert_eq!(resample(&s, 48_000, 48_000).len(), 4800);
    }
}
