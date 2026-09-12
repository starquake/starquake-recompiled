//! The display the game draws into.
//!
//! Starquake's logic reads the screen back (collision tests look at cell
//! attributes), so the display is game state, not just output. It is kept
//! in the Spectrum's native layout: a 6144-byte bitmap in the ULA's
//! interleaved line order, followed by 768 attribute bytes.

pub const BITMAP_LEN: usize = 6144;
pub const ATTR_LEN: usize = 768;
/// The display plus one guard row of attributes after it. Code that looks
/// at the cells around a sprite near the bottom edge reads and writes that
/// row, so it is kept too (the game fills it with `0x40` at the start).
pub const LEN: usize = BITMAP_LEN + ATTR_LEN + 32;

pub const WIDTH: usize = 256;
pub const HEIGHT: usize = 192;

const PALETTE: [u32; 16] = [
    0x000000, 0x0000D8, 0xD80000, 0xD800D8, 0x00D800, 0x00D8D8, 0xD8D800, 0xD8D8D8, //
    0x000000, 0x0000FF, 0xFF0000, 0xFF00FF, 0x00FF00, 0x00FFFF, 0xFFFF00, 0xFFFFFF,
];

#[derive(Clone)]
pub struct Display {
    pub mem: [u8; LEN],
    pub border: u8,
}

impl Default for Display {
    fn default() -> Self {
        Display {
            mem: [0; LEN],
            border: 0,
        }
    }
}

/// Offset of the first pixel line of character cell (`row`, `col`).
///
/// Reproduces the original's address arithmetic, including its wrapping
/// when `col` is past the right edge.
pub fn cell_offset(row: u8, col: u8) -> usize {
    let low = ((row & 7) << 5).wrapping_add(col);
    let high = row & 0x18;
    (high as usize) << 8 | low as usize
}

/// Offset of the attribute byte belonging to the bitmap byte at `offset`.
pub fn attr_offset_for(offset: usize) -> usize {
    BITMAP_LEN + (((offset >> 11) & 3) << 8) + (offset & 0xFF)
}

impl Display {
    /// Writes a character cell's bitmap and returns the offset of its attribute.
    pub fn put_cell(&mut self, row: u8, col: u8, pixels: &[u8; 8]) -> usize {
        let base = cell_offset(row, col);
        for (line, &p) in pixels.iter().enumerate() {
            self.mem[base + (line << 8)] = p;
        }
        attr_offset_for(base)
    }

    pub fn attr(&self, row: u8, col: u8) -> u8 {
        self.mem[BITMAP_LEN + row as usize * 32 + col as usize]
    }

    /// Blanks the room area (rows 6–23) and makes it bright white on black.
    pub fn clear_room_area(&mut self) {
        for y in 48..HEIGHT {
            let line = ((y & 0xC0) << 5) | ((y & 7) << 8) | ((y & 0x38) << 2);
            self.mem[line..line + 32].fill(0);
        }
        self.mem[BITMAP_LEN + 6 * 32..BITMAP_LEN + ATTR_LEN].fill(0x47);
    }

    /// Renders to 0RGB pixels, `WIDTH` × `HEIGHT`.
    pub fn render(&self, flash_phase: bool, out: &mut [u32]) {
        for y in 0..HEIGHT {
            let line = ((y & 0xC0) << 5) | ((y & 7) << 8) | ((y & 0x38) << 2);
            for col in 0..32 {
                let bits = self.mem[line + col];
                let attr = self.mem[BITMAP_LEN + (y / 8) * 32 + col];
                let bright = ((attr >> 6) & 1) as usize * 8;
                let mut ink = PALETTE[(attr & 7) as usize + bright];
                let mut paper = PALETTE[((attr >> 3) & 7) as usize + bright];
                if attr & 0x80 != 0 && flash_phase {
                    std::mem::swap(&mut ink, &mut paper);
                }
                for bit in 0..8 {
                    let on = bits & (0x80 >> bit) != 0;
                    out[y * WIDTH + col * 8 + bit] = if on { ink } else { paper };
                }
            }
        }
    }
}
