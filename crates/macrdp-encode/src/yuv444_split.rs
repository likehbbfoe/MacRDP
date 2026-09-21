//! YUV444 preprocessing for AVC444 dual-stream encoding.
//!
//! Implements BGRA → YUV444 conversion (BT.601) and the B-area pixel mapping
//! defined in MS-RDPEGFX §3.3.8.3.2 for splitting a YUV444 frame into two
//! standard YUV420 frames (Main View + Auxiliary View).

/// A standard YUV420 frame with separate Y, U, V planes.
pub struct Yuv420Frame {
    /// Luma plane, W × H bytes
    pub y: Vec<u8>,
    /// Chroma-blue plane, W/2 × H/2 bytes
    pub u: Vec<u8>,
    /// Chroma-red plane, W/2 × H/2 bytes
    pub v: Vec<u8>,
    pub width: u32,
    pub height: u32,
}

impl Yuv420Frame {
    /// Create a new zeroed frame with the given dimensions.
    /// Width and height should already be even (caller is responsible for alignment).
    pub fn new(width: u32, height: u32) -> Self {
        let y_size = (width * height) as usize;
        let uv_size = (width / 2 * height / 2) as usize;
        Self {
            y: vec![0u8; y_size],
            u: vec![128u8; uv_size],
            v: vec![128u8; uv_size],
            width,
            height,
        }
    }

    /// Ensure the frame is sized for the given dimensions, reallocating if needed.
    /// Resets all planes to neutral values (Y=0, U=V=128).
    pub fn ensure_size(&mut self, width: u32, height: u32) {
        let y_size = (width * height) as usize;
        let uv_size = (width / 2 * height / 2) as usize;

        if self.y.len() != y_size {
            self.y.resize(y_size, 0);
        }
        if self.u.len() != uv_size {
            self.u.resize(uv_size, 128);
            self.v.resize(uv_size, 128);
        }

        self.y.fill(0);
        self.u.fill(128);
        self.v.fill(128);
        self.width = width;
        self.height = height;
    }
}

/// Convert a BGRA frame to full-resolution YUV444 planes using BT.601 **full range**.
///
/// The output planes `y444`, `u444`, `v444` must each be at least `width * height` bytes.
/// BGRA byte order: B[0], G[1], R[2], A[3].
///
/// BT.601 full range (Y: 0-255, UV: 0-255) — matches NV12 '420f' format:
/// ```text
/// Y =  (77*R + 150*G + 29*B) >> 8
/// U = (-43*R -  85*G + 128*B) >> 8 + 128
/// V = (128*R - 107*G -  21*B) >> 8 + 128
/// ```
pub fn bgra_to_yuv444(
    bgra: &[u8],
    width: u32,
    height: u32,
    stride: usize,
    y444: &mut [u8],
    u444: &mut [u8],
    v444: &mut [u8],
) {
    let w = width as usize;
    let h = height as usize;
    debug_assert!(y444.len() >= w * h);
    debug_assert!(u444.len() >= w * h);
    debug_assert!(v444.len() >= w * h);

    for row in 0..h {
        let bgra_row = row * stride;
        let yuv_row = row * w;
        for col in 0..w {
            let px = bgra_row + col * 4;
            let b = bgra[px] as i32;
            let g = bgra[px + 1] as i32;
            let r = bgra[px + 2] as i32;

            // BT.601 full range (no +16 offset on Y, full 0-255 range)
            let y = (77 * r + 150 * g + 29 * b) >> 8;
            let u = ((-43 * r - 85 * g + 128 * b) >> 8) + 128;
            let v = ((128 * r - 107 * g - 21 * b) >> 8) + 128;

            let idx = yuv_row + col;
            y444[idx] = y.clamp(0, 255) as u8;
            u444[idx] = u.clamp(0, 255) as u8;
            v444[idx] = v.clamp(0, 255) as u8;
        }
    }
}

/// Split YUV444 planes into Main View (standard YUV420) and Auxiliary View
/// (chroma compensation YUV420) per MS-RDPEGFX §3.3.8.3.2 B-area mapping.
///
/// Both `main_view` and `aux_view` must be pre-allocated to at least `width × height`.
/// Width and height must be even.
///
/// ## B-area mapping
///
/// **Main View:**
/// - B1: `main.y[row][col] = y444[row][col]` (identity copy)
/// - B2: `main.u[r][c] = avg(u444[2r][2c], u444[2r][2c+1], u444[2r+1][2c], u444[2r+1][2c+1])`
/// - B3: `main.v[r][c]` = same 2×2 average for V
///
/// **Auxiliary View:**
/// - B4-B5: `aux.y` — U444/V444 odd rows, interleaved in 16-row blocks (8 U + 8 V)
/// - B6: `aux.u[r][c] = u444[2r][2c+1]` (even rows, odd columns)
/// - B7: `aux.v[r][c] = v444[2r][2c+1]` (even rows, odd columns)
pub fn yuv444_split_to_yuv420(
    y444: &[u8],
    u444: &[u8],
    v444: &[u8],
    width: u32,
    height: u32,
    main_view: &mut Yuv420Frame,
    aux_view: &mut Yuv420Frame,
) {
    let w = width as usize;
    let h = height as usize;
    let half_w = w / 2;
    let half_h = h / 2;

    debug_assert!(w % 2 == 0 && h % 2 == 0, "width and height must be even");
    debug_assert!(main_view.y.len() >= w * h);
    debug_assert!(main_view.u.len() >= half_w * half_h);
    debug_assert!(aux_view.y.len() >= w * h);
    debug_assert!(aux_view.u.len() >= half_w * half_h);

    // --- B1: Main Y plane — direct copy from Y444 ---
    main_view.y[..w * h].copy_from_slice(&y444[..w * h]);

    // --- B2 + B3: Main U/V planes — 2×2 anti-alias average ---
    for r in 0..half_h {
        let row_top = 2 * r;
        let row_bot = row_top + 1;
        for c in 0..half_w {
            let col_left = 2 * c;
            let col_right = col_left + 1;

            let tl = row_top * w + col_left;
            let tr = row_top * w + col_right;
            let bl = row_bot * w + col_left;
            let br = row_bot * w + col_right;

            // B2: U average
            let u_avg = (u444[tl] as u16 + u444[tr] as u16 + u444[bl] as u16 + u444[br] as u16) / 4;
            main_view.u[r * half_w + c] = u_avg as u8;

            // B3: V average
            let v_avg = (v444[tl] as u16 + v444[tr] as u16 + v444[bl] as u16 + v444[br] as u16) / 4;
            main_view.v[r * half_w + c] = v_avg as u8;
        }
    }

    // --- B4-B5: Aux Y plane — U444/V444 odd rows, interleaved in 16-row blocks ---
    // Zero-fill aux Y first (unused rows stay 0)
    aux_view.y[..w * h].fill(0);

    // Iterate over odd source rows: 1, 3, 5, ...
    let mut src_row = 1usize;
    while src_row < h {
        let half_row = src_row / 2; // 0, 1, 2, ...
        let block = half_row / 8;
        let offset = half_row % 8;
        let u_dst_row = block * 16 + offset;       // U data in first 8 rows of block
        let v_dst_row = block * 16 + offset + 8;   // V data in next 8 rows

        let src_offset = src_row * w;
        // B4: Copy U444 odd row into aux Y
        if u_dst_row < h {
            let u_dst_offset = u_dst_row * w;
            aux_view.y[u_dst_offset..u_dst_offset + w]
                .copy_from_slice(&u444[src_offset..src_offset + w]);
        }
        // B5: Copy V444 odd row into aux Y
        if v_dst_row < h {
            let v_dst_offset = v_dst_row * w;
            aux_view.y[v_dst_offset..v_dst_offset + w]
                .copy_from_slice(&v444[src_offset..src_offset + w]);
        }

        src_row += 2;
    }

    // --- B6 + B7: Aux U/V planes — even rows, odd columns ---
    for r in 0..half_h {
        let src_row = 2 * r; // even row
        for c in 0..half_w {
            let src_col = 2 * c + 1; // odd column
            let src_idx = src_row * w + src_col;
            let dst_idx = r * half_w + c;

            // B6: Aux U
            aux_view.u[dst_idx] = u444[src_idx];
            // B7: Aux V
            aux_view.v[dst_idx] = v444[src_idx];
        }
    }
}
