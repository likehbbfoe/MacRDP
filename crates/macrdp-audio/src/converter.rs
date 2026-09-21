/// Audio format conversion utilities.
pub struct AudioConverter;

impl AudioConverter {
    /// Convert a slice of f32 samples (range [-1.0, 1.0]) to 16-bit signed little-endian PCM bytes.
    ///
    /// Each f32 sample is clamped to [-1.0, 1.0], scaled by 32767, then written as two LE bytes.
    pub fn float32_to_s16le(input: &[f32]) -> Vec<u8> {
        let mut output = Vec::with_capacity(input.len() * 2);
        for &sample in input {
            let clamped = sample.clamp(-1.0, 1.0);
            let scaled = (clamped * 32767.0) as i16;
            output.extend_from_slice(&scaled.to_le_bytes());
        }
        output
    }

    /// Interleave non-interleaved channel buffers.
    ///
    /// Given `buffers` where each element is a channel's samples, produce a single interleaved
    /// buffer of length `num_channels * num_samples`.
    ///
    /// Example: buffers = [[L0, L1, L2], [R0, R1, R2]] → [L0, R0, L1, R1, L2, R2]
    pub fn interleave(buffers: &[&[f32]], num_samples: usize) -> Vec<f32> {
        let num_channels = buffers.len();
        let mut output = Vec::with_capacity(num_channels * num_samples);
        for i in 0..num_samples {
            for channel in buffers {
                output.push(channel[i]);
            }
        }
        output
    }
}
