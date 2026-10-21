//! Dictation capture (spec 7): microphone only, kept in memory.

use std::sync::mpsc::{self, Receiver};

use anyhow::{anyhow, Result};

use crate::capture::{open_mic, NO_MIC, PERMISSION_DENIED};
use crate::dsp::{downmix, Resampler};

pub struct DictationRecorder {
    stream: cpal::Stream,
    rx: Receiver<Vec<f32>>,
    rate: u32,
    channels: u16,
}

impl DictationRecorder {
    /// Opens the default input device and starts recording. Errors carry a
    /// `capture.*` code as their message.
    pub fn start() -> Result<DictationRecorder> {
        let (tx, rx) = mpsc::channel();
        let o = open_mic(tx).map_err(|e| {
            log::warn!("dictation mic failed: {:?}", e.kind());
            anyhow!(if e.kind() == cpal::ErrorKind::PermissionDenied { PERMISSION_DENIED } else { NO_MIC })
        })?;
        Ok(DictationRecorder { stream: o.stream, rx, rate: o.rate, channels: o.channels })
    }

    /// Stops recording and returns 16 kHz mono samples.
    pub fn stop(self) -> Vec<f32> {
        drop(self.stream);
        let interleaved: Vec<f32> = self.rx.try_iter().flatten().collect();
        Resampler::new(self.rate).process(&downmix(&interleaved, self.channels))
    }
}
