//! The display the game draws into.
//!
//! Starquake's logic reads the screen back (collision tests look at cell
//! attributes), so the display is game state, not just output. It is kept
//! in the Spectrum's native layout: a 6144-byte bitmap in the ULA's
//! interleaved line order, followed by 768 attribute bytes.

pub use zx_core::screen::{ATTR_LEN, BITMAP_LEN};
/// The display plus one guard row of attributes after it. Code that looks
/// at the cells around a sprite near the bottom edge reads and writes that
/// row, so it is kept too (the game fills it with `0x40` at the start).
pub const LEN: usize = BITMAP_LEN + ATTR_LEN + 32;

pub use zx_core::screen::{HEIGHT, WIDTH, line_offset};

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

/// Offset into the attributes of the cell holding the pixel at (`x`, `y`),
/// with `y` counted up from the bottom of the screen as the game counts it.
///
/// The original writes this three ways — as an address, as an index, and
/// folded into a rotate — but they are the same ten bits. The wrapping is
/// the original's: a `y` past the bottom wraps rather than failing.
pub fn attr_index(x: u8, y: u8) -> usize {
    ((0xBFu8.wrapping_sub(y) & 0xF8) as usize) << 2 | (x >> 3) as usize
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
            let line = line_offset(y);
            self.mem[line..line + 32].fill(0);
        }
        self.mem[BITMAP_LEN + 6 * 32..BITMAP_LEN + ATTR_LEN].fill(0x47);
    }

    /// Renders to 0RGB pixels, `WIDTH` × `HEIGHT`, with no border.
    pub fn render(&self, flash_phase: bool, out: &mut [u32]) {
        zx_core::screen::render(&self.mem, &self.mem[BITMAP_LEN..], flash_phase, out, WIDTH, 0, |c| c);
    }
}
