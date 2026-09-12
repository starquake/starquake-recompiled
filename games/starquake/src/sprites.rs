//! Per-frame display work: moving sprites, their colours, and the small
//! animations (sparkles, crumbling platforms, force fields).

use crate::display::{ATTR_LEN, BITMAP_LEN};
use crate::entities::SLOTS;
use crate::game::Game;

/// Screen-line address table in the original (2 bytes per pixel row,
/// starting with row 16). Sprite slots hold pointers into it.
const LINE_TABLE: u16 = 0xDDDE;
/// Two-cell animation frames (16 bytes each) for platforms and force fields.
const STRIP_FRAMES: u16 = 0xDC55;

impl Game {
    fn ram_word(&self, addr: u16) -> u16 {
        let r = &self.assets.ram;
        r[addr as usize] as u16 | (r[addr.wrapping_add(1) as usize] as u16) << 8
    }

    /// XORs a 24 × 16 sprite onto the screen. `lines` points into the line
    /// table; `col` is the byte column. Bytes wrap within their 256-byte
    /// page, as in the original.
    fn xor_sprite(&mut self, lines: u16, graphic: u16, col: u8) {
        let mut g = graphic;
        for i in 0..16u16 {
            let line = self.ram_word(lines.wrapping_add(i * 2));
            let page = line & 0xFF00;
            let mut lo = (line as u8).wrapping_add(col);
            for _ in 0..3 {
                let addr = (page | lo as u16) as usize;
                // Rows past the bottom of the table read address 0: the
                // writes land in ROM and have no effect.
                if addr >= 0x4000 {
                    debug_assert!(addr < 0x5B00, "sprite line off screen");
                    self.display.mem[addr - 0x4000] ^= self.assets.ram[g as usize];
                }
                lo = lo.wrapping_add(1);
                g = g.wrapping_add(1);
            }
        }
    }

    /// Erases every sprite where it was last drawn and draws it at its
    /// current position (both by XOR).
    pub fn draw_sprites(&mut self) {
        for k in 0..SLOTS {
            let e = self.entities[k].0;
            self.xor_sprite(u16::from_le_bytes([e[0], e[1]]), u16::from_le_bytes([e[2], e[3]]), e[4]);

            let (x, y) = (e[5], e[6]);
            // Graphics are stored pre-shifted by 0, 2, 4 and 6 pixels.
            let graphic = u16::from_le_bytes([e[7], e[8]]).wrapping_add((x & 6) as u16 * 24);
            let lines = (0xAFu8.wrapping_sub(y) as u16 * 2).wrapping_add(LINE_TABLE);
            let col = x >> 3;
            let [l0, l1] = lines.to_le_bytes();
            let [g0, g1] = graphic.to_le_bytes();
            self.entities[k].0[..5].copy_from_slice(&[l0, l1, g0, g1, col]);
            self.xor_sprite(lines, graphic, col);
        }
    }

    /// Gives each sprite's cells its ink (cells with green-including paper
    /// are left alone), then re-applies the colours of bright scenery.
    pub fn colour_sprites(&mut self) {
        for k in 0..SLOTS {
            let e = self.entities[k].0;
            let (x, y, ink) = (e[5], e[6], e[9]);
            if (x | y) < 0x10 {
                continue;
            }
            let base = crate::display::attr_index(x, y);
            let narrow = x & 7 == 0;
            let short = y.wrapping_add(1) & 7 == 0;
            let mut mask = 0xF8u8;
            let paint = |g: &mut Game, i: usize, mask: u8| {
                let a = &mut g.display.mem[BITMAP_LEN + i];
                if *a & 0x20 == 0 {
                    *a = (*a & mask) | ink;
                }
            };

            paint(self, base, mask);
            paint(self, base + 1, mask);
            if !narrow {
                paint(self, base + 2, mask);
                // In this version of the game the check for this cell is a
                // stray RST 38: it runs the ROM's interrupt routine (one
                // extra frame count), always paints the cell, and leaves the
                // mask one higher for the rest of the sprite.
                self.frames = self.frames.wrapping_add(1);
                mask += 1;
                let a = &mut self.display.mem[BITMAP_LEN + base + 34];
                *a = (*a & mask) | ink;
            }
            paint(self, base + 33, mask);
            paint(self, base + 32, mask);
            if !short {
                paint(self, base + 64, mask);
                paint(self, base + 65, mask);
                if !narrow {
                    paint(self, base + 66, mask);
                }
            }
        }

        let mut p = 0;
        while p + 2 < self.restore_mem.len() {
            let (lo, hi) = (self.restore_mem[p], self.restore_mem[p + 1]);
            if hi == 0 {
                break;
            }
            let v = self.restore_mem[p + 2];
            let a = &mut self.display.mem[(u16::from_le_bytes([lo, hi]) - 0x4000) as usize];
            *a = (*a & 0xC0) | (v & 0x3F);
            p += 3;
        }
    }

    /// XORs a two-cell animation frame at (`row`, `col`) and sets its
    /// colour (bit 7 set: see [`Game::xor_cell`]).
    ///
    /// # Panics
    ///
    /// If the loaded game data is too short to hold what the original keeps
    /// there, which means the file was not Starquake.
    pub fn draw_strip(&mut self, frame: u8, row: u8, col: u8, attr: u8) {
        let at = STRIP_FRAMES.wrapping_add(frame.rotate_left(4) as u16) as usize;
        let pixels: [u8; 16] = self.assets.ram[at..at + 16].try_into().unwrap();
        self.xor_cell(row, col, &pixels[..8], attr | 0x80);
        self.xor_cell(row, col.wrapping_add(1), &pixels[8..], attr | 0x80);
    }

    /// Advances one of BLOB's platforms: they crumble through four stages
    /// once their timer runs low.
    pub fn tick_platforms(&mut self) {
        self.platform_cursor = if self.platform_cursor + 1 >= 12 { 0 } else { self.platform_cursor + 1 };
        let at = self.platform_cursor as usize * 4;
        let p = &mut self.platforms[at..at + 4];
        if p[1] == 0 {
            return;
        }
        let (col, row) = (p[0] & 0x1F, p[1] & 0x7F);
        p[3] = p[3].wrapping_sub(1);
        if p[3] < 4 {
            p[0] = p[0].wrapping_add(0x20);
            if p[0] & 0xE0 == 0x80 {
                p[1] = 0;
            }
        }
        let stage = p[0] >> 5;
        if stage == 0 {
            return;
        }
        let attr = if stage == 4 { 0x40 } else { 0x00 };
        self.draw_strip(stage - 1, row, col, attr);
    }

    /// Blinks one sparkle cell pair.
    pub fn tick_sparkles(&mut self) {
        let [mut e, mut d] = self.objects.sparkle_cursor.to_le_bytes();
        e = e.wrapping_add(1);
        let (wrapped, carry) = e.overflowing_add(0xD0);
        if carry {
            e = wrapped;
            d ^= 7;
        }
        self.objects.sparkle_cursor = u16::from_le_bytes([e, d]);
        let at = e.rotate_left(1) as usize;
        let (col, row) = (self.objects.sparkle_table[at], self.objects.sparkle_table[at + 1]);
        let b = row.rotate_right(3);
        let low = (b & 0xE0) | col;
        if low == 0 {
            return;
        }
        let i = ((b & 3) as usize) << 8 | low as usize;
        debug_assert!(i + 1 < ATTR_LEN + 32);
        self.display.mem[BITMAP_LEN + i] = d;
        self.display.mem[BITMAP_LEN + i + 1] = d;
    }

    /// Advances one force field: it switches on and off with its period,
    /// and flickers in random colours while on.
    pub fn tick_force_fields(&mut self) {
        let next = self.objects.force_field_cursor.wrapping_add(1);
        let cursor = if next >= 4 { 0 } else { next };
        self.objects.force_field_cursor = cursor;
        let at = cursor as usize * 8;
        let rec = &mut self.objects.force_fields[at..at + 6];
        let (col, row) = (rec[0], rec[1]);
        if row == 0 {
            return;
        }
        rec[3] = rec[3].wrapping_sub(1);
        let timer = rec[3];
        if timer == 0xFF {
            rec[3] = rec[4];
            rec[5] ^= 1;
            self.draw_strip(5, row, col, 0x47);
            return;
        }
        if rec[5] == 0 {
            return;
        }
        let frame = [6, 7, 7, 6][(timer & 3) as usize];
        let attr = (self.rng.lo() & 3) + 0x44;
        self.draw_strip(frame, row, col, attr);
    }
}
