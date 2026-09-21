//! Video encoding abstraction (H.264 via VideoToolbox / OpenH264)

mod openh264_enc;
mod settings;
pub mod yuv444_split;
#[cfg(target_os = "macos")]
pub mod color_convert;
#[cfg(target_os = "macos")]
mod videotoolbox;

use anyhow::Result;
use bytes::Bytes;

pub use openh264_enc::OpenH264Encoder;
pub use settings::{validate_dimensions, VideoSettings, MAX_DIMENSION, MAX_PIXELS};
#[cfg(target_os = "macos")]
pub use videotoolbox::VtEncoder;

/// Encoded frame output
pub struct EncodedFrame {
    /// H.264 NAL units (Annex B format)
    pub data: Bytes,
    /// Whether this is a key frame (IDR)
    pub is_keyframe: bool,
    pub width: u32,
    pub height: u32,
}

/// AVC444 dual-stream encoded result
pub struct Avc444EncodedFrame {
    /// Stream1: Main YUV420 H.264 (luma + downsampled chroma)
    pub main_view: EncodedFrame,
    /// Stream2: Auxiliary YUV420 H.264 (chroma compensation)
    pub aux_view: EncodedFrame,
}

/// Quality preset
#[derive(Debug, Clone, Copy)]
pub enum Quality {
    LowLatency,
    Balanced,
    HighQuality,
}

/// Video encoder trait
pub trait VideoEncoder: Send {
    /// AVC420 encode (existing)
    fn encode_bgra(&mut self, data: &[u8], width: u32, height: u32, stride: usize) -> Result<EncodedFrame>;

    /// AVC444 dual-stream encode.
    /// Internally performs BGRA -> YUV444 -> B-area split -> dual session encode.
    fn encode_bgra_444(&mut self, data: &[u8], width: u32, height: u32, stride: usize) -> Result<Avc444EncodedFrame>;

    /// Zero-copy encode from CVPixelBuffer (NV12). Default returns error (unsupported).
    /// Only VtEncoder overrides this.
    fn encode_pixel_buffer(&mut self, _ptr: *mut std::ffi::c_void, _force_keyframe: bool) -> Result<EncodedFrame> {
        anyhow::bail!("encode_pixel_buffer not supported by this encoder")
    }

    /// Whether the selected encoder accepts captured NV12 CVPixelBuffers directly.
    fn supports_pixel_buffer(&self) -> bool { false }

    /// Whether the selected backend is guaranteed to use hardware acceleration.
    fn is_hardware_accelerated(&self) -> bool { false }

    fn set_bitrate(&mut self, bitrate_bps: u32);
    fn force_keyframe(&mut self);

    /// Whether this encoder supports AVC444 dual-stream encoding
    fn supports_444(&self) -> bool;

    /// Whether this encoder supports async submit/collect pipelining.
    fn supports_pipelining(&self) -> bool { false }

    /// Submit a BGRA frame for async encoding. Returns immediately.
    fn submit_bgra(&mut self, _data: &[u8], _width: u32, _height: u32, _stride: usize) -> Result<()> {
        anyhow::bail!("submit_bgra not supported by this encoder")
    }

    /// Submit a CVPixelBuffer for async encoding. Returns immediately.
    fn submit_pixel_buffer(&mut self, _ptr: *mut std::ffi::c_void) -> Result<()> {
        anyhow::bail!("submit_pixel_buffer not supported by this encoder")
    }

    /// Collect result from a previous submit. Blocks up to timeout.
    fn collect_encoded(&mut self, _timeout: std::time::Duration) -> Result<Option<EncodedFrame>> {
        Ok(None)
    }
}

/// Select capture input from the initialized encoder, including software fallback.
/// AVC444 needs BGRA for its chroma split; bitmap fallback also needs BGRA.
pub fn capture_uses_nv12(encoder: Option<&dyn VideoEncoder>, mode_444: bool) -> bool {
    !mode_444 && encoder.is_some_and(VideoEncoder::supports_pixel_buffer)
}

/// Align a dimension up to the nearest multiple of 16 (H.264 macroblock size)
pub fn align16(v: u32) -> u32 {
    (v + 15) & !15
}

/// Calculate optimal bitrate for screen content
pub fn screen_bitrate(width: u32, height: u32, fps: f32, quality: Quality) -> u32 {
    let pixels = width as f64 * height as f64;
    let base_bpp = match quality {
        Quality::LowLatency => 8.0,
        Quality::Balanced => 16.0,
        Quality::HighQuality => 24.0,
    };
    let fps_factor = (fps as f64 / 30.0).max(1.0);
    (pixels * base_bpp * fps_factor).clamp(100_000.0, 1_000_000_000.0) as u32
}

/// Encoder preference
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum EncoderPreference {
    /// OpenH264 CPU encoding with accelerated color conversion where available.
    Software,
    /// VideoToolbox media hardware, with software fallback if initialization fails.
    Hardware,
    /// Prefer available hardware and fall back to OpenH264.
    Auto,
}

impl EncoderPreference {
    pub fn from_str_opt(s: Option<&str>) -> Self {
        Self::try_from_str_opt(s).unwrap_or(Self::Auto)
    }

    pub fn try_from_str_opt(s: Option<&str>) -> Result<Self> {
        match s.map(|s| s.trim().to_ascii_lowercase()).as_deref() {
            Some("hardware" | "gpu" | "videotoolbox" | "vt") => Ok(Self::Hardware),
            Some("software" | "cpu" | "openh264" | "oh264") => Ok(Self::Software),
            None | Some("auto") => Ok(Self::Auto),
            _ => anyhow::bail!("encoder must be auto, hardware or software; HEVC and AV1 are not RDP output modes"),
        }
    }

    pub fn as_str(self) -> &'static str {
        match self {
            Self::Auto => "auto",
            Self::Hardware => "hardware",
            Self::Software => "software",
        }
    }
}

/// Create an H.264 encoder based on preference.
/// When `mode_444` is true, the encoder will initialize dual sessions for AVC444 support.
pub fn create_encoder(width: u32, height: u32, fps: f32, _quality: Quality, preference: EncoderPreference, mode_444: bool, bitrate: u32) -> Result<Box<dyn VideoEncoder>> {
    validate_dimensions(width, height)?;
    anyhow::ensure!(fps.is_finite() && fps >= 1.0 && fps <= 120.0, "Encoder frame rate must be between 1 and 120");
    anyhow::ensure!(bitrate > 0 && bitrate <= 1_000_000_000, "Encoder bitrate is out of range");
    let enc_w = align16(width);
    let enc_h = align16(height);

    create_with_fallback(
        preference,
        || {
            #[cfg(target_os = "macos")]
            {
                let encoder = VtEncoder::new(enc_w, enc_h, fps, bitrate, mode_444)?;
                tracing::info!(enc_w, enc_h, mode_444, "Using verified VideoToolbox hardware encoder");
                Ok(Box::new(encoder))
            }
            #[cfg(not(target_os = "macos"))]
            anyhow::bail!("VideoToolbox requires macOS")
        },
        || {
            tracing::info!(enc_w, enc_h, mode_444, "Using OpenH264 software encoder (CPU)");
            Ok(Box::new(OpenH264Encoder::new(enc_w, enc_h, fps, bitrate, mode_444)?))
        },
    )
}

fn create_with_fallback(
    preference: EncoderPreference,
    hardware: impl FnOnce() -> Result<Box<dyn VideoEncoder>>,
    software: impl FnOnce() -> Result<Box<dyn VideoEncoder>>,
) -> Result<Box<dyn VideoEncoder>> {
    if preference != EncoderPreference::Software {
        match hardware() {
            Ok(encoder) => return Ok(encoder),
            Err(e) => {
                tracing::warn!("VideoToolbox unavailable: {e}, falling back to OpenH264");
            }
        }
    }
    software()
}
