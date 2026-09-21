//! Color-aware conversion and scaling for RDP bitmap frames.

use std::ffi::c_void;
use std::ptr;

use anyhow::{ensure, Context, Result};
use bytes::Bytes;
use core_graphics::base::{kCGBitmapByteOrder32Little, kCGImageAlphaPremultipliedFirst};
use core_graphics::color_space::{kCGColorSpaceSRGB, CGColorSpace};
use core_graphics::context::{CGContext, CGInterpolationQuality};
use core_graphics::geometry::{CGPoint, CGRect, CGSize};
use core_graphics::image::CGImage;
use core_graphics::sys;
use foreign_types::ForeignType;

use crate::SafePixelBuffer;

pub(crate) fn validate_dimensions(width: u32, height: u32) -> Result<()> {
    ensure!(
        (1..=8192).contains(&width) && (1..=8192).contains(&height),
        "Capture dimensions must be between 1 and 8192 pixels per side"
    );
    ensure!(
        u64::from(width) * u64::from(height) <= 7680 * 4320,
        "Capture dimensions exceed the maximum pixel count"
    );
    Ok(())
}

pub(crate) fn pixel_buffer_to_bgra(
    buffer: &SafePixelBuffer,
    width: u32,
    height: u32,
) -> Result<(Bytes, usize)> {
    validate_dimensions(width, height)?;
    let mut image_ptr = ptr::null_mut();
    // SAFETY: buffer owns a retained CVPixelBuffer and remains alive and unchanged
    // until the derived image has been rendered. The API returns a retained image.
    let status =
        unsafe { VTCreateCGImageFromCVPixelBuffer(buffer.as_ptr(), ptr::null(), &mut image_ptr) };
    let image = if image_ptr.is_null() {
        None
    } else {
        // SAFETY: the non-null output follows the CoreFoundation create rule.
        Some(unsafe { CGImage::from_ptr(image_ptr) })
    };
    ensure!(
        status == 0,
        "Pixel-buffer image conversion failed: {status}"
    );
    let image = image.context("Pixel-buffer image conversion returned no image")?;
    ensure!(
        image.width() == width as usize && image.height() == height as usize,
        "Pixel-buffer dimensions do not match frame metadata"
    );
    // VideoToolbox reads the buffer's color attachments; CoreGraphics then
    // converts that color space to sRGB instead of assuming a fixed YUV matrix.
    render_bgra(&image, width, height)
}

pub(crate) fn render_bgra(image: &CGImage, width: u32, height: u32) -> Result<(Bytes, usize)> {
    validate_dimensions(width, height)?;
    let stride = (width as usize)
        .checked_mul(4)
        .context("BGRA stride overflow")?;
    let length = stride
        .checked_mul(height as usize)
        .context("BGRA size overflow")?;
    let mut pixels = Vec::new();
    pixels
        .try_reserve_exact(length)
        .context("Unable to allocate BGRA frame")?;
    pixels.resize(length, 0u8);
    let color_space = CGColorSpace::create_with_name(unsafe { kCGColorSpaceSRGB })
        .context("Unable to create sRGB color space")?;
    // SAFETY: pixels has length stride * height and remains allocated until
    // context is explicitly dropped. 32-bit little-endian ARGB is byte-order BGRA.
    let context_ptr = unsafe {
        CGBitmapContextCreate(
            pixels.as_mut_ptr().cast(),
            width as usize,
            height as usize,
            8,
            stride,
            color_space.as_ptr(),
            kCGImageAlphaPremultipliedFirst | kCGBitmapByteOrder32Little,
        )
    };
    ensure!(
        !context_ptr.is_null(),
        "Unable to create BGRA bitmap context"
    );
    // SAFETY: the non-null context is a newly created, owned reference.
    let context = unsafe { CGContext::from_ptr(context_ptr) };
    context.set_interpolation_quality(CGInterpolationQuality::CGInterpolationQualityHigh);
    context.draw_image(
        CGRect::new(
            &CGPoint::new(0.0, 0.0),
            &CGSize::new(width as f64, height as f64),
        ),
        image,
    );
    context.flush();
    drop(context);
    Ok((Bytes::from(pixels), stride))
}

#[link(name = "VideoToolbox", kind = "framework")]
extern "C" {
    fn VTCreateCGImageFromCVPixelBuffer(
        pixel_buffer: *mut c_void,
        options: *const c_void,
        image_out: *mut *mut sys::CGImage,
    ) -> i32;
}

#[link(name = "CoreGraphics", kind = "framework")]
extern "C" {
    fn CGBitmapContextCreate(
        data: *mut c_void,
        width: usize,
        height: usize,
        bits_per_component: usize,
        bytes_per_row: usize,
        color_space: *mut sys::CGColorSpace,
        bitmap_info: u32,
    ) -> *mut sys::CGContext;
}
