//! Sound outputs (speakers, headsets, HDMI…) recorded into `system.wav`. Every active
//! output is captured: a call app may play on a different one than the default. On macOS
//! a single global tap covers every output at once.

use anyhow::Result;
use barnaby_core::OutputDevice;

/// A captured output; dropping it stops the capture.
pub(crate) struct Source {
    pub rate: u32,
    pub channels: u16,
    #[cfg(not(target_os = "linux"))]
    _stream: cpal::Stream,
    #[cfg(target_os = "macos")]
    _tap: platform::Tap,
    #[cfg(target_os = "linux")]
    parec: std::process::Child,
}

#[cfg(not(any(target_os = "linux", target_os = "macos")))]
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

    /// Opens `id` in loopback mode (how WASAPI captures an output).
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

/// macOS 14.2+: one source, a global Core Audio process tap (every process, every output,
/// mixed to stereo) carried by an aggregate device that cpal reads like a microphone. Not
/// cpal's per-output loopback: on an output that also has an input (AirPods, USB headsets)
/// cpal records that input, the user's mic, instead of the output.
#[cfg(target_os = "macos")]
mod platform {
    use super::*;
    use std::ffi::CStr;
    use std::ptr::NonNull;

    use anyhow::{bail, Context};
    use cpal::traits::{DeviceTrait, HostTrait};
    use objc2::rc::Retained;
    use objc2::runtime::AnyObject;
    use objc2::AnyThread;
    use objc2_core_audio::{
        kAudioAggregateDeviceIsPrivateKey, kAudioAggregateDeviceNameKey, kAudioAggregateDeviceTapAutoStartKey,
        kAudioAggregateDeviceTapListKey, kAudioAggregateDeviceUIDKey, kAudioSubTapDriftCompensationKey, kAudioSubTapUIDKey,
        AudioHardwareCreateAggregateDevice, AudioHardwareCreateProcessTap, AudioHardwareDestroyAggregateDevice,
        AudioHardwareDestroyProcessTap, AudioObjectID, CATapDescription, CATapMuteBehavior,
    };
    use objc2_core_foundation::CFDictionary;
    use objc2_foundation::{NSArray, NSDictionary, NSNumber, NSString};

    const ID: &str = "system";

    pub fn list() -> Vec<OutputDevice> {
        vec![OutputDevice { id: ID.into(), name: "Tout le son de l'ordinateur".into() }]
    }

    /// The tap and its aggregate device; dropped after the stream that reads them.
    pub(crate) struct Tap {
        tap: AudioObjectID,
        aggregate: AudioObjectID,
    }

    impl Drop for Tap {
        fn drop(&mut self) {
            // SAFETY: both ids were created by this process and are destroyed once.
            unsafe {
                AudioHardwareDestroyAggregateDevice(self.aggregate);
                AudioHardwareDestroyProcessTap(self.tap);
            }
        }
    }

    pub(crate) fn open(_id: &str, on_data: impl FnMut(Vec<f32>) + Send + 'static) -> Result<Source> {
        // A private aggregate should be listed to its own process; if this macOS hides it,
        // a public one (visible to other apps while recording) is the fallback.
        let (tap, device) = match aggregate(true)? {
            (tap, Some(device)) => (tap, device),
            (tap, None) => {
                drop(tap);
                match aggregate(false)? {
                    (tap, Some(device)) => (tap, device),
                    _ => bail!("system audio device not found"),
                }
            }
        };
        let config = device.default_input_config()?;
        let o = crate::input::open(&device, config, on_data)?;
        Ok(Source { rate: o.rate, channels: o.channels, _stream: o.stream, _tap: tap })
    }

    /// Creates the tap (every process, unmuted) and an aggregate device carrying it,
    /// then finds that device through cpal by its UID.
    fn aggregate(private: bool) -> Result<(Tap, Option<cpal::Device>)> {
        let uid = format!("app.barnaby.system-audio.{}.{}", std::process::id(), u8::from(private));
        // SAFETY: plain Core Audio calls on objects created here; every status is checked.
        let tap = unsafe {
            let desc = CATapDescription::initStereoGlobalTapButExcludeProcesses(CATapDescription::alloc(), &NSArray::new());
            desc.setName(&NSString::from_str("Barnaby"));
            desc.setPrivate(true);
            desc.setMuteBehavior(CATapMuteBehavior::Unmuted);
            let mut tap: AudioObjectID = 0;
            check(AudioHardwareCreateProcessTap(Some(&desc), &mut tap), "tap")?;

            let true_ = NSNumber::new_bool(true);
            let sub_tap: Retained<NSDictionary<NSString, AnyObject>> = NSDictionary::from_slices(
                &[&*key(kAudioSubTapUIDKey), &*key(kAudioSubTapDriftCompensationKey)],
                &[obj(&*desc.UUID().UUIDString()), obj(&*true_)],
            );
            let props: Retained<NSDictionary<NSString, AnyObject>> = NSDictionary::from_slices(
                &[
                    &*key(kAudioAggregateDeviceNameKey),
                    &*key(kAudioAggregateDeviceUIDKey),
                    &*key(kAudioAggregateDeviceTapListKey),
                    &*key(kAudioAggregateDeviceTapAutoStartKey),
                    &*key(kAudioAggregateDeviceIsPrivateKey),
                ],
                &[
                    obj(&*NSString::from_str("Barnaby system audio")),
                    obj(&*NSString::from_str(&uid)),
                    obj(&*NSArray::from_retained_slice(&[sub_tap])),
                    obj(&*true_),
                    obj(&*NSNumber::new_bool(private)),
                ],
            );
            // NSDictionary is toll-free bridged to CFDictionary.
            let props = &*(Retained::as_ptr(&props).cast::<CFDictionary>());
            let mut aggregate: AudioObjectID = 0;
            let status = AudioHardwareCreateAggregateDevice(props, NonNull::from(&mut aggregate));
            if status != 0 {
                AudioHardwareDestroyProcessTap(tap);
            }
            check(status, "aggregate")?;
            Tap { tap, aggregate }
        };
        let id = cpal::DeviceId::new(cpal::HostId::CoreAudio, &uid);
        Ok((tap, cpal::default_host().device_by_id(&id)))
    }

    fn obj<T: AsRef<AnyObject>>(t: &T) -> &AnyObject {
        t.as_ref()
    }

    fn key(k: &CStr) -> Retained<NSString> {
        NSString::from_str(k.to_str().unwrap_or_default())
    }

    fn check(status: i32, what: &str) -> Result<()> {
        if status == 0 { Ok(()) } else { Err(anyhow::anyhow!("core audio {what} failed: {status}")).context("system_audio") }
    }

    impl Source {
        pub fn alive(&mut self) -> bool {
            true
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
