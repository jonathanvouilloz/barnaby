//! Sample conversion shared by meeting capture and dictation.

use barnaby_core::SAMPLE_RATE;

pub fn downmix(interleaved: &[f32], channels: u16) -> Vec<f32> {
    let n = usize::from(channels.max(1));
    interleaved.chunks_exact(n).map(|frame| frame.iter().sum::<f32>() / n as f32).collect()
}

pub fn rms(samples: &[f32]) -> f32 {
    if samples.is_empty() {
        return 0.0;
    }
    (samples.iter().map(|s| s * s).sum::<f32>() / samples.len() as f32).sqrt().min(1.0)
}

pub fn to_i16(sample: f32) -> i16 {
    (sample.clamp(-1.0, 1.0) * f32::from(i16::MAX)) as i16
}

/// Streaming linear resampler to 16 kHz: state carries over between chunks so
/// chunk boundaries do not click.
// shortcut: no anti-alias filter before decimation, fine for speech; add a low-pass if quality suffers.
pub struct Resampler {
    step: f64,
    /// Position of the next output sample, in input samples from the current chunk start
    /// (-1 = the last sample of the previous chunk).
    pos: f64,
    prev: f32,
}

impl Resampler {
    pub fn new(input_rate: u32) -> Self {
        Self { step: f64::from(input_rate) / f64::from(SAMPLE_RATE), pos: 0.0, prev: 0.0 }
    }

    pub fn process(&mut self, input: &[f32]) -> Vec<f32> {
        if self.step == 1.0 {
            return input.to_vec();
        }
        if input.is_empty() {
            return Vec::new();
        }
        let last = (input.len() - 1) as f64;
        let mut out = Vec::with_capacity((input.len() as f64 / self.step) as usize + 1);
        while self.pos < last {
            let i = self.pos.floor();
            let frac = (self.pos - i) as f32;
            let a = if i < 0.0 { self.prev } else { input[i as usize] };
            let b = input[(i + 1.0) as usize];
            out.push(a + (b - a) * frac);
            self.pos += self.step;
        }
        self.pos -= input.len() as f64;
        self.prev = input[input.len() - 1];
        out
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn downmix_averages_channels() {
        assert_eq!(downmix(&[1.0, 0.0, 0.5, 0.5, -1.0, 1.0], 2), vec![0.5, 0.5, 0.0]);
        assert_eq!(downmix(&[0.1, 0.2], 1), vec![0.1, 0.2]);
    }

    #[test]
    fn resample_48k_keeps_duration_across_chunks() {
        let mut r = Resampler::new(48_000);
        let input: Vec<f32> = (0..48_000).map(|i| i as f32 / 48_000.0).collect();
        let out: Vec<f32> = input.chunks(479).flat_map(|c| r.process(c)).collect();
        assert!((15_999..=16_001).contains(&out.len()), "{}", out.len());
        // A ramp stays a ramp: every output sample is the input at 3x its index.
        for (k, s) in out.iter().enumerate() {
            assert!((s - (3 * k) as f32 / 48_000.0).abs() < 1e-4, "sample {k}");
        }
    }

    #[test]
    fn resample_upsamples_and_handles_44k1() {
        let mut r = Resampler::new(8_000);
        let out = r.process(&[0.0, 1.0, 0.0, 1.0]);
        assert_eq!(out, vec![0.0, 0.5, 1.0, 0.5, 0.0, 0.5]);
        assert_eq!(Resampler::new(44_100).process(&vec![0.0; 44_100]).len(), 16_000);
    }

    #[test]
    fn i16_conversion_clamps() {
        assert_eq!(to_i16(2.0), i16::MAX);
        assert_eq!(to_i16(-2.0), -i16::MAX);
        assert_eq!(to_i16(0.0), 0);
    }
}
