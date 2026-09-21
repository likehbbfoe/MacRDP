//! Rebuild complete stream settings when changing capture cadence.

use anyhow::Result;
use screencapturekit::prelude::*;
use screencapturekit::stream::configuration::{AudioChannelCount, AudioSampleRate};

use crate::{bitmap, CaptureConfig, CapturePixelFormat};

pub(crate) fn validate_frame_rate(fps: u32) -> Result<()> {
    anyhow::ensure!(
        (1..=120).contains(&fps),
        "Capture frame rate must be between 1 and 120"
    );
    Ok(())
}

pub(crate) fn stream_configuration(
    config: &CaptureConfig,
    captures_audio: bool,
    fps: u32,
) -> Result<SCStreamConfiguration> {
    bitmap::validate_dimensions(config.width, config.height)?;
    validate_frame_rate(fps)?;
    Ok(SCStreamConfiguration::new()
        .with_width(config.width)
        .with_height(config.height)
        .with_scales_to_fit(true)
        .with_minimum_frame_interval(&CMTime::new(1, fps as i32))
        .with_pixel_format(match config.pixel_format {
            CapturePixelFormat::Nv12 => PixelFormat::YCbCr_420f,
            CapturePixelFormat::Bgra => PixelFormat::BGRA,
        })
        .with_shows_cursor(config.show_cursor)
        .with_captures_audio(captures_audio)
        .with_sample_rate(AudioSampleRate::Rate48000)
        .with_channel_count(AudioChannelCount::Stereo)
        .with_excludes_current_process_audio(true))
}
