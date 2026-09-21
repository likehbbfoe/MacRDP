use anyhow::{ensure, Result};

use crate::EncoderPreference;

/// Bounds shared by capture configuration and H.264 session creation.
pub const MAX_DIMENSION: u32 = 8192;
pub const MAX_PIXELS: u64 = 7680 * 4320;

pub fn validate_dimensions(width: u32, height: u32) -> Result<()> {
    ensure!(width > 0 && height > 0, "Display dimensions must be nonzero");
    ensure!(width <= MAX_DIMENSION && height <= MAX_DIMENSION,
        "Display dimensions must not exceed {MAX_DIMENSION} pixels per side");
    ensure!(u64::from(width) * u64::from(height) <= MAX_PIXELS,
        "Display dimensions exceed the supported pixel count");
    Ok(())
}

/// User-facing video options, validated before allocating capture or encoder resources.
pub struct VideoSettings<'a> {
    pub encoder: Option<&'a str>,
    pub chroma_mode: Option<&'a str>,
    pub quality: Option<&'a str>,
    pub width: u32,
    pub height: u32,
    pub frame_rate: u32,
    pub bitrate_mbps: Option<u32>,
    pub resolution: Option<&'a str>,
}

impl VideoSettings<'_> {
    pub fn validate(&self) -> Result<()> {
        EncoderPreference::try_from_str_opt(self.encoder)?;
        ensure!(matches!(self.chroma_mode, None | Some("avc420" | "avc444")),
            "chroma_mode must be avc420 or avc444");
        ensure!(matches!(self.quality, None | Some("low_latency" | "balanced" | "high_quality")),
            "quality must be low_latency, balanced or high_quality");
        ensure!((1..=120).contains(&self.frame_rate), "frame_rate must be between 1 and 120");
        if let Some(bitrate) = self.bitrate_mbps {
            ensure!((1..=1000).contains(&bitrate), "bitrate_mbps must be between 1 and 1000");
        }
        if self.width != 0 || self.height != 0 {
            validate_dimensions(self.width, self.height)?;
        }
        match self.resolution {
            None | Some("auto") => {}
            Some(value) if value.contains('x') => {
                let (width, height) = value.split_once('x').expect("separator checked");
                validate_dimensions(width.parse()?, height.parse()?)?;
            }
            Some(value) => {
                let scale: u32 = value.parse().map_err(|_| anyhow::anyhow!(
                    "resolution must be auto, WIDTHxHEIGHT or a scale from 1 to 4"))?;
                ensure!((1..=4).contains(&scale), "resolution scale must be between 1 and 4");
            }
        }
        Ok(())
    }
}
