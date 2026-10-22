//! Sound outputs (speakers, headsets, HDMI…) recorded into `system.wav`. Every active
//! output is captured: a call app may play on a different one than the default.

use anyhow::Result;
use barnaby_core::OutputDevice;

/// A captured output; dropping it stops the capture.
pub(crate) struct Source {
    pub rate: u32,
    pub channels: u16,
    #[cfg(not(target_os = "linux"))]
    _stream: cpal::Stream,
    #[cfg(target_os = "linux")]
    parec: std::process::Child,
}

#[cfg(not(target_os = "linux"))]
mod platform {
    use super::*;
    use cpal::traits::{DeviceTrait, HostTrait};

    /// Active outputs only (WASAPI enumerates `DEVICE_STATE_ACTIVE` endpoints).
    pub fn list() -> Vec<OutputDevice> {
        let Ok(devices) = cpal::default_host().output_devices() else { return Vec::new() };
        devices
            .filter_map(|d| {
                let id = d.id().ok()?.to_string();
                let name = d.description().map(|desc| desc.name().to_owned()).unwrap_or_else(|_| id.clone());
                Some(OutputDevice { id, name })
            })
            .collect()
    }

    /// Opens `id` in loopback mode (how WASAPI and CoreAudio capture an output).
    pub(crate) fn open(id: &str, on_data: impl FnMut(Vec<f32>) + Send + 'static) -> Result<Source> {
        let device = cpal::default_host()
            .device_by_id(&id.parse()?)
            .ok_or(cpal::Error::new(cpal::ErrorKind::DeviceNotAvailable))?;
        let config = device.default_output_config()?;
        let o = crate::input::open(&device, config, on_data)?;
        Ok(Source { rate: o.rate, channels: o.channels, _stream: o.stream })
    }

    impl Source {
        pub fn alive(&mut self) -> bool {
            true
        }
    }
}

/// PulseAudio / PipeWire: one `parec` per sink monitor, already 16 kHz mono.
#[cfg(target_os = "linux")]
mod platform {
    use super::*;
    use barnaby_core::SAMPLE_RATE;
    use std::io::Read;
    use std::process::{Command, Stdio};
    use std::thread;

    pub fn list() -> Vec<OutputDevice> {
        // `pactl` translates its labels: force the C locale.
        match Command::new("pactl").args(["list", "sinks"]).env("LC_ALL", "C").output() {
            Ok(o) if o.status.success() => parse_sinks(&String::from_utf8_lossy(&o.stdout)),
            Ok(_) => Vec::new(),
            Err(e) => {
                log::warn!("pactl failed: {e}");
                Vec::new()
            }
        }
    }

    pub(super) fn parse_sinks(text: &str) -> Vec<OutputDevice> {
        let mut out: Vec<OutputDevice> = Vec::new();
        for line in text.lines().map(str::trim) {
            if let Some(name) = line.strip_prefix("Name: ") {
                out.push(OutputDevice { id: name.into(), name: name.into() });
            } else if let (Some(desc), Some(last)) = (line.strip_prefix("Description: "), out.last_mut()) {
                last.name = desc.into();
            }
        }
        out
    }

    pub(crate) fn open(id: &str, mut on_data: impl FnMut(Vec<f32>) + Send + 'static) -> Result<Source> {
        let mut parec = Command::new("parec")
            .args([&format!("--device={id}.monitor"), "--format=s16le", "--rate=16000", "--channels=1", "--raw"])
            .stdout(Stdio::piped())
            .stderr(Stdio::null())
            .spawn()?;
        let Some(mut out) = parec.stdout.take() else {
            anyhow::bail!("parec stdout not piped");
        };
        thread::spawn(move || {
            let mut buf = [0u8; 3200]; // 100 ms
            while out.read_exact(&mut buf).is_ok() {
                on_data(buf.as_chunks::<2>().0.iter().map(|&b| f32::from(i16::from_le_bytes(b)) / 32768.0).collect());
            }
        });
        Ok(Source { rate: SAMPLE_RATE, channels: 1, parec })
    }

    impl Source {
        /// A `parec` that exited (sink gone, sound server restarted) is reopened by the next scan.
        pub fn alive(&mut self) -> bool {
            matches!(self.parec.try_wait(), Ok(None))
        }
    }

    impl Drop for Source {
        fn drop(&mut self) {
            let _ = self.parec.kill();
            let _ = self.parec.wait();
        }
    }
}

pub use platform::list;
pub(crate) use platform::open;

#[cfg(all(test, target_os = "linux"))]
mod tests {
    use super::platform::parse_sinks;

    #[test]
    fn parses_pactl_sinks() {
        let text = "Sink #0\n\tState: SUSPENDED\n\tName: alsa_output.pci-0000_00_1f.3.analog-stereo\n\tDescription: Built-in Audio Analog Stereo\n\tDriver: PipeWire\n\tProperties:\n\t\tdevice.description = \"Built-in Audio\"\n\nSink #52\n\tState: RUNNING\n\tName: bluez_output.AA_BB_CC_DD_EE_FF.1\n\tDescription: WH-1000XM4\n";
        let sinks = parse_sinks(text);
        let pairs: Vec<_> = sinks.iter().map(|s| (s.id.as_str(), s.name.as_str())).collect();
        assert_eq!(
            pairs,
            [("alsa_output.pci-0000_00_1f.3.analog-stereo", "Built-in Audio Analog Stereo"), ("bluez_output.AA_BB_CC_DD_EE_FF.1", "WH-1000XM4")]
        );
    }
}
