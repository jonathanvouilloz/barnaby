//! cpal input streams in any device format, delivered as interleaved f32 chunks.

use cpal::traits::{DeviceTrait, StreamTrait};
use cpal::{Device, Error, FromSample, SampleFormat, SizedSample, Stream, SupportedStreamConfig};

pub struct Opened {
    pub stream: Stream,
    pub rate: u32,
    pub channels: u16,
}

/// Opens and starts `device` at its default config. Output devices are opened in
/// loopback mode by WASAPI (and CoreAudio), which is how system audio is captured.
pub fn open(
    device: &Device,
    config: SupportedStreamConfig,
    on_data: impl FnMut(Vec<f32>) + Send + 'static,
) -> Result<Opened, Error> {
    let (rate, channels) = (config.sample_rate(), config.channels());
    let cfg = config.config();
    let stream = match config.sample_format() {
        SampleFormat::I8 => build::<i8>(device, cfg, on_data),
        SampleFormat::I16 => build::<i16>(device, cfg, on_data),
        SampleFormat::I32 => build::<i32>(device, cfg, on_data),
        SampleFormat::U8 => build::<u8>(device, cfg, on_data),
        SampleFormat::U16 => build::<u16>(device, cfg, on_data),
        SampleFormat::U32 => build::<u32>(device, cfg, on_data),
        SampleFormat::F64 => build::<f64>(device, cfg, on_data),
        _ => build::<f32>(device, cfg, on_data),
    }?;
    stream.play()?;
    Ok(Opened { stream, rate, channels })
}

fn build<T>(device: &Device, cfg: cpal::StreamConfig, mut on_data: impl FnMut(Vec<f32>) + Send + 'static) -> Result<Stream, Error>
where
    T: SizedSample,
    f32: FromSample<T>,
{
    device.build_input_stream::<T, _, _>(
        cfg,
        move |data: &[T], _: &cpal::InputCallbackInfo| on_data(data.iter().map(|s| s.to_sample::<f32>()).collect()),
        |e| log::warn!("audio stream error: {:?}", e.kind()),
        None,
    )
}
