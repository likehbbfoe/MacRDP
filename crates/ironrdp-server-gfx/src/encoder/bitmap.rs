use ironrdp_core::{cast_int, cast_length, invalid_field_err, Encode as _, WriteCursor};
use ironrdp_graphics::image_processing::PixelFormat;
use ironrdp_graphics::rdp6::{
    ABgrChannels, ARgbChannels, BgrAChannels, BitmapEncodeError, BitmapStreamEncoder, RgbAChannels,
};
use ironrdp_pdu::bitmap::{self, BitmapData, BitmapUpdateData, Compression};
use ironrdp_pdu::geometry::InclusiveRectangle;

use crate::BitmapUpdate;

// PERF: we could also remove the need for this buffer
#[derive(Clone)]
pub(crate) struct BitmapEncoder {
    buffer: Vec<u8>,
}

impl BitmapEncoder {
    pub(crate) fn new() -> Self {
        Self {
            buffer: vec![0; usize::from(u16::MAX)],
        }
    }

    pub(crate) fn encode(&mut self, bitmap: &BitmapUpdate, output: &mut [u8]) -> Result<usize, BitmapEncodeError> {
        let bytes_per_pixel = u16::from(bitmap.format.bytes_per_pixel());
        // RDP6 bitmap scanlines are aligned to four pixels. Pad the encoded
        // image by repeating its last pixel, while retaining the visible bounds.
        let aligned_width: u16 = cast_int!("aligned width", (u32::from(bitmap.width.get()) + 3) & !3)
            .map_err(BitmapEncodeError::Encode)?;
        let row_len = aligned_width.checked_mul(bytes_per_pixel)
            .ok_or_else(|| BitmapEncodeError::Encode(invalid_field_err!("bitmap", "scanline is too wide")))?;
        let chunk_height = u16::MAX / row_len;

        let mut cursor = WriteCursor::new(output);
        let stride = bitmap.stride.get();
        let visible_row_len = usize::from(bitmap.width.get()) * usize::from(bytes_per_pixel);
        let required_len = usize::from(bitmap.height.get() - 1).checked_mul(stride)
            .and_then(|offset| offset.checked_add(visible_row_len));
        if stride < visible_row_len || required_len.is_none_or(|len| len > bitmap.data.len()) {
            return Err(BitmapEncodeError::Encode(invalid_field_err!("bitmap", "invalid pixel buffer layout")));
        }

        let total = bitmap.height.get().div_ceil(chunk_height);
        BitmapUpdateData::encode_header(total, &mut cursor).map_err(BitmapEncodeError::Encode)?;

        for start_row in (0..bitmap.height.get()).step_by(usize::from(chunk_height)) {
            let height = (bitmap.height.get() - start_row).min(chunk_height);
            let top = bitmap.y + start_row;

            let encoder = BitmapStreamEncoder::new(usize::from(aligned_width), usize::from(height));

            let len = {
                let pixels = (start_row..start_row + height)
                    .rev()
                    .flat_map(|y| {
                        let row = &bitmap.data[usize::from(y) * stride..];
                        (0..usize::from(aligned_width)).map(move |x| {
                            let offset = x.min(usize::from(bitmap.width.get()) - 1)
                                * usize::from(bytes_per_pixel);
                            &row[offset..offset + usize::from(bytes_per_pixel)]
                        })
                    });

                Self::encode_iter(encoder, bitmap.format, pixels, self.buffer.as_mut_slice())?
            };

            let data = BitmapData {
                rectangle: InclusiveRectangle {
                    left: bitmap.x,
                    top,
                    right: bitmap.x + bitmap.width.get() - 1,
                    bottom: top + height - 1,
                },
                width: aligned_width,
                height,
                bits_per_pixel: u16::from(bitmap.format.bytes_per_pixel()) * 8,
                compression_flags: Compression::BITMAP_COMPRESSION,
                compressed_data_header: Some(bitmap::CompressedDataHeader {
                    main_body_size: cast_length!("main body size", len).map_err(BitmapEncodeError::Encode)?,
                    scan_width: aligned_width,
                    uncompressed_size: height * row_len,
                }),
                bitmap_data: &self.buffer[..len],
            };

            data.encode(&mut cursor).map_err(BitmapEncodeError::Encode)?;
        }

        Ok(cursor.pos())
    }

    fn encode_iter<'a, P>(
        mut encoder: BitmapStreamEncoder,
        format: PixelFormat,
        src: P,
        dst: &mut [u8],
    ) -> Result<usize, BitmapEncodeError>
    where
        P: Iterator<Item = &'a [u8]> + Clone,
    {
        let written = match format {
            PixelFormat::ARgb32 | PixelFormat::XRgb32 => {
                encoder.encode_pixels_stream::<_, ARgbChannels>(src, dst, true)?
            }
            PixelFormat::RgbA32 | PixelFormat::RgbX32 => {
                encoder.encode_pixels_stream::<_, RgbAChannels>(src, dst, true)?
            }
            PixelFormat::ABgr32 | PixelFormat::XBgr32 => {
                encoder.encode_pixels_stream::<_, ABgrChannels>(src, dst, true)?
            }
            PixelFormat::BgrA32 | PixelFormat::BgrX32 => {
                encoder.encode_pixels_stream::<_, BgrAChannels>(src, dst, true)?
            }
        };

        Ok(written)
    }
}
