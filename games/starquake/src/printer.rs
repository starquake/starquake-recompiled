//! Text output.
//!
//! The original prints all its text through the Spectrum ROM's PRINT
//! routine, with strings that embed control codes. This is a
//! reimplementation of the parts of that behaviour the game relies on:
//!
//! - `0x16 row col` (AT) moves the print position.
//! - `0x10`–`0x13 n` (INK, PAPER, FLASH, BRIGHT) set *temporary* colours:
//!   an attribute value plus a mask of bits to keep from the screen
//!   (`n = 8`, "transparent").
//! - `0x14 n` / `0x15 n` (INVERSE, OVER) invert glyphs / XOR them onto the
//!   screen.
//! - Printing past column 31 wraps to the next line on the next character.
//!
//! Colour state persists from one string to the next, as it does in the
//! original.

use crate::display::{self, Display};

/// Glyphs for characters `0x20`–`0x7F`.
pub type Font = [[u8; 8]; 96];

/// The block graphics, characters `0x80`–`0x8F`. The ROM builds these from
/// the low four bits rather than from a table: bit 0 is the top right
/// quarter of the cell, then top left, bottom right, bottom left.
fn block_graphic(n: u8) -> [u8; 8] {
    let half = |bits: u8| (if bits & 1 != 0 { 0x0F } else { 0 }) | (if bits & 2 != 0 { 0xF0 } else { 0 });
    let (top, bottom) = (half(n), half(n >> 2));
    [top, top, top, top, bottom, bottom, bottom, bottom]
}

const P_OVER: u8 = 0x01;
const P_INVERSE: u8 = 0x04;
const P_INK9: u8 = 0x10;
const P_PAPER9: u8 = 0x40;

#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
enum Pending {
    #[default]
    None,
    /// A colour or OVER/INVERSE control code waiting for its operand.
    Operand(u8),
    AtRow,
    AtCol(u8),
}

#[derive(Clone, Debug, Default, PartialEq, Eq)]
pub struct Printer {
    pub row: u8,
    /// 0–31, or 32 when the next character wraps to the next line.
    pub col: u8,
    /// Temporary attribute.
    pub attr_t: u8,
    /// Attribute bits taken from the screen instead of `attr_t`.
    pub mask_t: u8,
    /// OVER / INVERSE / contrast flags.
    pub p_flag: u8,
    pending: Pending,
}

impl Printer {
    pub fn at(row: u8, col: u8, attr_t: u8, mask_t: u8, p_flag: u8) -> Printer {
        Printer {
            row,
            col,
            attr_t,
            mask_t,
            p_flag,
            pending: Pending::None,
        }
    }

    pub fn print(&mut self, d: &mut Display, font: &Font, udg: &[[u8; 8]], text: &[u8]) {
        for &b in text {
            self.put(d, font, udg, b);
        }
    }

    pub fn put(&mut self, d: &mut Display, font: &Font, udg: &[[u8; 8]], b: u8) {
        match self.pending {
            Pending::Operand(code) => {
                self.pending = Pending::None;
                self.control(code, b);
                return;
            }
            Pending::AtRow => {
                self.pending = Pending::AtCol(b);
                return;
            }
            Pending::AtCol(row) => {
                self.pending = Pending::None;
                debug_assert!(row < 24 && b < 32, "AT {row},{b} is off screen");
                self.row = row;
                self.col = b;
                return;
            }
            Pending::None => {}
        }
        match b {
            0x08 => self.backspace(),
            0x10..=0x15 => self.pending = Pending::Operand(b),
            0x16 => self.pending = Pending::AtRow,
            0x20..=0x7F => self.glyph(d, &font[(b - 0x20) as usize]),
            0x80..=0x8F => self.glyph(d, &block_graphic(b - 0x80)),
            0x90..=0xA4 => self.glyph(d, &udg[(b - 0x90) as usize]),
            _ => panic!("unsupported print code {b:#04x}"),
        }
    }

    /// Moves the print position one place back.
    fn backspace(&mut self) {
        if self.col == 0 {
            self.col = 31;
            self.row = self.row.wrapping_sub(1);
        } else {
            self.col -= 1;
        }
    }

    fn control(&mut self, code: u8, n: u8) {
        match code {
            0x10 | 0x11 => {
                let (bits, value, contrast) = if code == 0x10 {
                    (0x07, n & 7, P_INK9)
                } else {
                    (0x38, (n & 7) << 3, P_PAPER9)
                };
                match n {
                    0..=7 => {
                        self.attr_t = (self.attr_t & !bits) | value;
                        self.mask_t &= !bits;
                        self.p_flag &= !contrast;
                    }
                    8 => {
                        self.mask_t |= bits;
                        self.p_flag &= !contrast;
                    }
                    _ => {
                        self.mask_t |= bits;
                        self.p_flag |= contrast;
                    }
                }
            }
            0x12 | 0x13 => {
                let bit = if code == 0x12 { 0x80 } else { 0x40 };
                match n {
                    0 | 1 => {
                        self.attr_t = (self.attr_t & !bit) | if n == 1 { bit } else { 0 };
                        self.mask_t &= !bit;
                    }
                    _ => self.mask_t |= bit,
                }
            }
            _ => {
                let bit = if code == 0x14 { P_INVERSE } else { P_OVER };
                self.p_flag = (self.p_flag & !bit) | if n & 1 != 0 { bit } else { 0 };
            }
        }
    }

    fn glyph(&mut self, d: &mut Display, glyph: &[u8; 8]) {
        if self.col >= 32 {
            self.col = 0;
            self.row += 1;
        }
        let base = display::cell_offset(self.row, self.col);
        let invert = if self.p_flag & P_INVERSE != 0 { 0xFF } else { 0 };
        let over = self.p_flag & P_OVER != 0;
        for (line, &g) in glyph.iter().enumerate() {
            let at = base + (line << 8);
            let old = if over { d.mem[at] } else { 0 };
            d.mem[at] = g ^ invert ^ old;
        }

        let at = display::attr_offset_for(base);
        let old = d.mem[at];
        let mut attr = (old & self.mask_t) | (self.attr_t & !self.mask_t);
        if self.p_flag & P_PAPER9 != 0 {
            attr &= 0xC7;
            if attr & 0x04 == 0 {
                attr ^= 0x38;
            }
        }
        if self.p_flag & P_INK9 != 0 {
            attr &= 0xF8;
            if attr & 0x20 == 0 {
                attr ^= 0x07;
            }
        }
        d.mem[at] = attr;
        self.col += 1;
    }
}
