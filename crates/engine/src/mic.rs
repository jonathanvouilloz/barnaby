//! Dictation capture (spec 7): microphone only, kept in memory.

use std::sync::mpsc::{self, Receiver};

use anyhow::{anyhow, Result};

use crate::capture::{open_mic, NO_MIC, PERMISSION_DENIED};
use crate::dsp::{downmix, Resampler};

pub struct DictationRecorder {
    // Recording lasts as long as the stream lives.
    _stream: cpal::Stream,
    rx: Receiver<Vec<f32>>,
    resampler: Resampler,
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
        Ok(DictationRecorder { _stream: o.stream, rx, resampler: Resampler::new(o.rate), channels: o.channels })
    }

    /// 16 kHz mono samples recorded since the last call.
    pub fn drain(&mut self) -> Vec<f32> {
        let interleaved: Vec<f32> = self.rx.try_iter().flatten().collect();
        self.resampler.process(&downmix(&interleaved, self.channels))
    }

    /// Stops recording and returns the samples not drained yet.
    pub fn stop(mut self) -> Vec<f32> {
        self.drain()
    }
}
