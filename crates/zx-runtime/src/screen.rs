//! ULA display rendering.

use crate::machine::Zx;

pub const BORDER: usize = 32;
pub const WIDTH: usize = 256 + 2 * BORDER;
pub const HEIGHT: usize = 192 + 2 * BORDER;

const PALETTE: [u32; 16] = [
    0x000000, 0x0000D8, 0xD80000, 0xD800D8, 0x00D800, 0x00D8D8, 0xD8D800, 0xD8D8D8, //
    0x000000, 0x0000FF, 0xFF0000, 0xFF00FF, 0x00FF00, 0x00FFFF, 0xFFFF00, 0xFFFFFF,
];

/// Renders the screen memory and border into a `WIDTH` x `HEIGHT` 0RGB buffer.
pub fn render(z: &Zx, out: &mut [u32]) {
    let border = PALETTE[z.border as usize];
    out.fill(border);
    let flash_phase = (z.frame / 16) % 2 == 1;
    for y in 0..192usize {
        let pixel_row = 0x4000 | ((y & 0xC0) << 5) | ((y & 7) << 8) | ((y & 0x38) << 2);
        let attr_row = 0x5800 + (y / 8) * 32;
        let line = &mut out[(y + BORDER) * WIDTH + BORDER..][..256];
        for col in 0..32 {
            let bits = z.mem[pixel_row + col];
            let attr = z.mem[attr_row + col];
            let bright = ((attr >> 6) & 1) as usize * 8;
            let mut ink = PALETTE[(attr & 7) as usize + bright];
            let mut paper = PALETTE[((attr >> 3) & 7) as usize + bright];
            if attr & 0x80 != 0 && flash_phase {
                std::mem::swap(&mut ink, &mut paper);
            }
            for bit in 0..8 {
                line[col * 8 + bit] = if bits & (0x80 >> bit) != 0 { ink } else { paper };
            }
        }
    }
}
