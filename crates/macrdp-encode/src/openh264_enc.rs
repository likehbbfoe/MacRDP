//! OpenH264 software H.264 encoder optimized for screen content

use std::ffi::c_void;

use anyhow::{Context, Result};
use bytes::Bytes;
use openh264::encoder::{Encoder, EncoderConfig, FrameType};
use openh264::formats::YUVSlices;

use crate::color_convert::VImageConverter;
use crate::{Avc444EncodedFrame, EncodedFrame, Quality, VideoEncoder};

pub struct OpenH264Encoder {
    encoder: Encoder,
    /// Auxiliary encoder for AVC444 chroma stream
    encoder_aux: Option<Encoder>,
    width: u32,
    height: u32,
    force_keyframe: bool,
    yuv_buf: Vec<u8>,
    /// Reused visible-area conversion buffer for macroblock-padded frames.
    visible_yuv_buf: Vec<u8>,
    /// Current target bitrate
    target_bitrate: u32,
    mode_444: bool,
    /// vImage SIMD accelerated BGRA→I420 converter (macOS Accelerate.framework)
    vimage: Option<VImageConverter>,
    /// Reusable buffers for AVC444 YUV444 split
    yuv444_bufs: Option<Yuv444SplitBufs>,
}

/// Reusable buffers for AVC444 encoding
struct Yuv444SplitBufs {
    y444: Vec<u8>,
    u444: Vec<u8>,
    v444: Vec<u8>,
    main_view: crate::yuv444_split::Yuv420Frame,
    aux_view: crate::yuv444_split::Yuv420Frame,
}

impl Yuv444SplitBufs {
    fn new(width: u32, height: u32) -> Self {
        let full = (width * height) as usize;
        Self {
            y444: vec![0u8; full],
            u444: vec![0u8; full],
            v444: vec![0u8; full],
            main_view: crate::yuv444_split::Yuv420Frame::new(width, height),
            aux_view: crate::yuv444_split::Yuv420Frame::new(width, height),
        }
    }
}

/// Calculate optimal bitrate for screen content
fn screen_bitrate(width: u32, height: u32, fps: f32, quality: Quality) -> u32 {
    let pixels = width as f64 * height as f64;
    // Screen content needs high bitrate for sharp text and UI edges
    // Base bits-per-pixel at 30fps, scaled by actual fps
    let base_bpp = match quality {
        Quality::LowLatency => 8.0,   // ~16 Mbps for 1080p@30
        Quality::Balanced => 16.0,    // ~33 Mbps for 1080p@30
        Quality::HighQuality => 24.0, // ~50 Mbps for 1080p@30
    };
    let fps_factor = (fps as f64 / 30.0).max(1.0);
    (pixels * base_bpp * fps_factor) as u32
}

fn create_oh264_encoder(_width: u32, _height: u32, fps: f32, bitrate: u32) -> Result<Encoder> {
    let config = EncoderConfig::new()
        .bitrate(openh264::encoder::BitRate::from_bps(bitrate))
        .max_frame_rate(openh264::encoder::FrameRate::from_hz(fps))
        .rate_control_mode(openh264::encoder::RateControlMode::Quality)
        .background_detection(false)
        .adaptive_quantization(true)
        .qp(openh264::encoder::QpRange::new(20, 40))
        .skip_frames(false)
        .usage_type(openh264::encoder::UsageType::ScreenContentRealTime)
        .complexity(openh264::encoder::Complexity::Medium)
        .intra_frame_period(openh264::encoder::IntraFramePeriod::from_num_frames(
            fps as u32 * 5,
        ))
        .long_term_reference(true)
        .num_threads(4);

    Encoder::with_api_config(openh264::OpenH264API::from_source(), config)
        .context("Failed to create OpenH264 encoder")
}

impl OpenH264Encoder {
    pub fn new(width: u32, height: u32, fps: f32, bitrate: u32, mode_444: bool) -> Result<Self> {
        let encoder = create_oh264_encoder(width, height, fps, bitrate)?;

        // AVC444: create auxiliary encoder with 70% bitrate
        let encoder_aux = if mode_444 {
            let aux_bitrate = (bitrate as f64 * 0.7) as u32;
            let aux = create_oh264_encoder(width, height, fps, aux_bitrate)?;
            tracing::info!(
                aux_bitrate_mbps = aux_bitrate as f64 / 1_000_000.0,
                "AVC444 auxiliary OpenH264 encoder created"
            );
            Some(aux)
        } else {
            None
        };

        let vimage = VImageConverter::new()
            .map_err(|e| tracing::warn!("vImage init failed, using scalar fallback: {e}"))
            .ok();

        let yuv_size = (width * height * 3 / 2) as usize;

        let yuv444_bufs = if mode_444 {
            Some(Yuv444SplitBufs::new(width, height))
        } else {
            None
        };

        tracing::info!(
            width,
            height,
            fps,
            mode_444,
            bitrate_mbps = bitrate as f64 / 1_000_000.0,
            "OpenH264 encoder created (screen-optimized)"
        );

        Ok(Self {
            encoder,
            encoder_aux,
            width,
            height,
            force_keyframe: false,
            yuv_buf: vec![0u8; yuv_size],
            visible_yuv_buf: vec![0u8; yuv_size],
            target_bitrate: bitrate,
            mode_444,
            vimage,
            yuv444_bufs,
        })
    }
}

/// Convert BGRA to YUV420 into an encoder-sized buffer (handles padding).
fn bgra_to_yuv420_padded(
    bgra: &[u8],
    src_w: u32,
    src_h: u32,
    stride: usize,
    enc_w: u32,
    enc_h: u32,
    yuv: &mut [u8],
) {
    let ew = enc_w as usize;
    let sw = src_w as usize;
    let sh = src_h as usize;
    let y_plane_size = ew * enc_h as usize;
    let uv_w = ew / 2;

    let (y_plane, uv_planes) = yuv.split_at_mut(y_plane_size);
    let uv_plane_size = uv_w * (enc_h as usize / 2);
    let (u_plane, v_plane) = uv_planes.split_at_mut(uv_plane_size);

    for row in 0..sh {
        for col in 0..sw {
            let bgra_offset = row * stride + col * 4;
            if bgra_offset + 2 >= bgra.len() {
                continue;
            }
            let b = bgra[bgra_offset] as i32;
            let g = bgra[bgra_offset + 1] as i32;
            let r = bgra[bgra_offset + 2] as i32;

            let y = ((66 * r + 129 * g + 25 * b + 128) >> 8) + 16;
            y_plane[row * ew + col] = y.clamp(0, 255) as u8;

            if row % 2 == 0 && col % 2 == 0 {
                let u = ((-38 * r - 74 * g + 112 * b + 128) >> 8) + 128;
                let v = ((112 * r - 94 * g - 18 * b + 128) >> 8) + 128;
                let uv_idx = (row / 2) * uv_w + (col / 2);
                u_plane[uv_idx] = u.clamp(0, 255) as u8;
                v_plane[uv_idx] = v.clamp(0, 255) as u8;
            }
        }
    }
}

/// Validate before calling native conversion routines or indexing pixel planes.
fn validate_bgra(
    data: &[u8],
    width: u32,
    height: u32,
    stride: usize,
    enc_w: u32,
    enc_h: u32,
) -> Result<()> {
    anyhow::ensure!(width > 0 && height > 0, "Frame dimensions must be positive");
    anyhow::ensure!(
        width <= enc_w && height <= enc_h,
        "Frame exceeds encoder dimensions"
    );
    let row_bytes = (width as usize)
        .checked_mul(4)
        .ok_or_else(|| anyhow::anyhow!("Frame row size overflow"))?;
    anyhow::ensure!(stride >= row_bytes, "BGRA stride is smaller than one row");
    let required = (height as usize - 1)
        .checked_mul(stride)
        .and_then(|offset| offset.checked_add(row_bytes))
        .ok_or_else(|| anyhow::anyhow!("BGRA buffer size overflow"))?;
    anyhow::ensure!(data.len() >= required, "BGRA buffer is truncated");
    Ok(())
}

/// Copy only visible I420 pixels, retaining the neutral macroblock border.
fn copy_i420_padded(
    src: &[u8],
    width: usize,
    height: usize,
    dst: &mut [u8],
    enc_w: usize,
    enc_h: usize,
) {
    let src_y = width * height;
    let dst_y = enc_w * enc_h;
    for (src_start, dst_start, cols, rows, pitch) in [
        (0, 0, width, height, enc_w),
        (src_y, dst_y, width / 2, height / 2, enc_w / 2),
        (
            src_y * 5 / 4,
            dst_y * 5 / 4,
            width / 2,
            height / 2,
            enc_w / 2,
        ),
    ] {
        for row in 0..rows {
            let input = src_start + row * cols;
            let output = dst_start + row * pitch;
            dst[output..output + cols].copy_from_slice(&src[input..input + cols]);
        }
    }
}

fn i420_slices(data: &[u8], width: u32, height: u32) -> YUVSlices<'_> {
    let (w, h) = (width as usize, height as usize);
    let (y, uv) = data.split_at(w * h);
    let (u, v) = uv.split_at(w * h / 4);
    YUVSlices::new((y, u, v), (w, h), (w, w / 2, w / 2))
}

fn split_frame_slices(frame: &crate::yuv444_split::Yuv420Frame) -> YUVSlices<'_> {
    let w = frame.width as usize;
    YUVSlices::new(
        (&frame.y, &frame.u, &frame.v),
        (w, frame.height as usize),
        (w, w / 2, w / 2),
    )
}

impl VideoEncoder for OpenH264Encoder {
    fn encode_bgra(
        &mut self,
        data: &[u8],
        width: u32,
        height: u32,
        stride: usize,
    ) -> Result<EncodedFrame> {
        validate_bgra(data, width, height, stride, self.width, self.height)?;

        // Zero the padding area
        let y_size = (self.width * self.height) as usize;
        let uv_size = (self.width / 2 * self.height / 2) as usize;
        self.yuv_buf[..y_size].fill(0);
        self.yuv_buf[y_size..y_size + uv_size].fill(128);
        self.yuv_buf[y_size + uv_size..].fill(128);

        // Accelerate even visible dimensions, including 1920x1080 -> 1920x1088.
        // Convert into a reusable compact plane layout then copy into aligned rows.
        if let Some(converter) = self
            .vimage
            .as_ref()
            .filter(|_| width % 2 == 0 && height % 2 == 0)
        {
            if width == self.width && height == self.height {
                converter
                    .bgra_to_i420(data, width, height, stride, &mut self.yuv_buf)
                    .map_err(anyhow::Error::msg)?;
            } else {
                converter
                    .bgra_to_i420(data, width, height, stride, &mut self.visible_yuv_buf)
                    .map_err(anyhow::Error::msg)?;
                copy_i420_padded(
                    &self.visible_yuv_buf,
                    width as usize,
                    height as usize,
                    &mut self.yuv_buf,
                    self.width as usize,
                    self.height as usize,
                );
            }
        } else {
            bgra_to_yuv420_padded(
                data,
                width,
                height,
                stride,
                self.width,
                self.height,
                &mut self.yuv_buf,
            );
        }
        let yuv = i420_slices(&self.yuv_buf, self.width, self.height);

        // Force IDR on next frame if requested
        if self.force_keyframe {
            self.encoder.force_intra_frame();
            self.force_keyframe = false;
        }

        let bitstream = self
            .encoder
            .encode(&yuv)
            .context("OpenH264 encode failed")?;

        let mut nal_data = Vec::new();
        bitstream.write_vec(&mut nal_data);

        let is_keyframe = matches!(bitstream.frame_type(), FrameType::IDR | FrameType::I);

        Ok(EncodedFrame {
            data: Bytes::from(nal_data),
            is_keyframe,
            width: self.width,
            height: self.height,
        })
    }

    fn encode_bgra_444(
        &mut self,
        data: &[u8],
        width: u32,
        height: u32,
        stride: usize,
    ) -> Result<Avc444EncodedFrame> {
        validate_bgra(data, width, height, stride, self.width, self.height)?;
        let encoder_aux = self
            .encoder_aux
            .as_mut()
            .ok_or_else(|| anyhow::anyhow!("AVC444 not enabled: no auxiliary encoder"))?;
        let bufs = self
            .yuv444_bufs
            .as_mut()
            .ok_or_else(|| anyhow::anyhow!("AVC444 not enabled: no YUV444 buffers"))?;

        let w = self.width;
        let h = self.height;

        // Keep encoder-sized row pitch even when the visible frame is smaller.
        bufs.y444.fill(0);
        bufs.u444.fill(128);
        bufs.v444.fill(128);
        for row in 0..height as usize {
            let dst = row * w as usize;
            crate::yuv444_split::bgra_to_yuv444(
                &data[row * stride..],
                width,
                1,
                stride,
                &mut bufs.y444[dst..],
                &mut bufs.u444[dst..],
                &mut bufs.v444[dst..],
            );
        }
        if self.force_keyframe {
            self.encoder.force_intra_frame();
            encoder_aux.force_intra_frame();
            self.force_keyframe = false;
        }

        // Step 2: YUV444 -> Main YUV420 + Aux YUV420 (B-area split)
        crate::yuv444_split::yuv444_split_to_yuv420(
            &bufs.y444,
            &bufs.u444,
            &bufs.v444,
            w,
            h,
            &mut bufs.main_view,
            &mut bufs.aux_view,
        );

        // Borrow reusable planes directly; no full-frame clone or repack.
        let main_yuv = split_frame_slices(&bufs.main_view);
        let main_bitstream = self
            .encoder
            .encode(&main_yuv)
            .context("OpenH264 main encode failed")?;
        let mut main_nal = Vec::new();
        main_bitstream.write_vec(&mut main_nal);
        let main_keyframe = matches!(main_bitstream.frame_type(), FrameType::IDR | FrameType::I);

        let aux_yuv = split_frame_slices(&bufs.aux_view);
        let aux_bitstream = encoder_aux
            .encode(&aux_yuv)
            .context("OpenH264 aux encode failed")?;
        let mut aux_nal = Vec::new();
        aux_bitstream.write_vec(&mut aux_nal);
        let aux_keyframe = matches!(aux_bitstream.frame_type(), FrameType::IDR | FrameType::I);

        tracing::debug!(
            main_bytes = main_nal.len(),
            aux_bytes = aux_nal.len(),
            main_keyframe,
            aux_keyframe,
            "AVC444 OpenH264 dual-stream encode complete"
        );

        Ok(Avc444EncodedFrame {
            main_view: EncodedFrame {
                data: Bytes::from(main_nal),
                is_keyframe: main_keyframe,
                width: w,
                height: h,
            },
            aux_view: EncodedFrame {
                data: Bytes::from(aux_nal),
                is_keyframe: aux_keyframe,
                width: w,
                height: h,
            },
        })
    }

    fn set_bitrate(&mut self, bitrate_bps: u32) {
        if self.target_bitrate == bitrate_bps {
            return;
        }
        self.target_bitrate = bitrate_bps;
        unsafe {
            let raw = self.encoder.raw_api();
            let mut bitrate_info = openh264_sys2::SBitrateInfo {
                iLayer: openh264_sys2::SPATIAL_LAYER_ALL as i32,
                iBitrate: bitrate_bps as i32,
            };
            let ret = raw.set_option(
                openh264_sys2::ENCODER_OPTION_BITRATE,
                &mut bitrate_info as *mut _ as *mut c_void,
            );
            if ret != 0 {
                tracing::warn!(
                    ret,
                    "OpenH264 set_option(BITRATE) failed, bitrate change deferred"
                );
            } else {
                tracing::info!(
                    bitrate_mbps = bitrate_bps as f64 / 1_000_000.0,
                    "Bitrate updated via SetOption"
                );
            }
        }
        if let Some(ref mut aux) = self.encoder_aux {
            let aux_bitrate = (bitrate_bps as f64 * 0.7) as u32;
            unsafe {
                let raw = aux.raw_api();
                let mut bitrate_info = openh264_sys2::SBitrateInfo {
                    iLayer: openh264_sys2::SPATIAL_LAYER_ALL as i32,
                    iBitrate: aux_bitrate as i32,
                };
                let _ = raw.set_option(
                    openh264_sys2::ENCODER_OPTION_BITRATE,
                    &mut bitrate_info as *mut _ as *mut c_void,
                );
            }
        }
    }

    fn force_keyframe(&mut self) {
        self.force_keyframe = true;
    }

    fn supports_444(&self) -> bool {
        self.mode_444
    }
}
