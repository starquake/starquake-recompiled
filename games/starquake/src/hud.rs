//! The status panel in the top six rows: frame, score, lives, the three
//! bars and the four inventory slots.

use crate::game::Game;

#[derive(Clone, Debug, Default, PartialEq, Eq)]
pub struct Status {
    /// Score digits, most significant first.
    pub score: [u8; 6],
    /// Points waiting to be added to each digit.
    pub pending: [u8; 6],
    pub lives: u8,
    /// The bars on rows 1–3, 0–127.
    pub bars: [u8; 3],
    /// Inventory slots: graphic number and colour (0 = empty).
    pub inventory: [(u8, u8); 4],
    /// Item just picked up, entering the inventory.
    pub incoming: (u8, u8),
    /// Item pushed out of the inventory, to be dropped.
    pub outgoing: (u8, u8),
}

/// Builds a byte string for the printer.
#[derive(Default)]
struct Text(Vec<u8>);

impl Text {
    fn at(mut self, row: u8, col: u8) -> Self {
        self.0.extend([0x16, row, col]);
        self
    }
    fn ink(mut self, n: u8) -> Self {
        self.0.extend([0x10, n]);
        self
    }
    fn bright(mut self, n: u8) -> Self {
        self.0.extend([0x13, n]);
        self
    }
    fn chars(mut self, s: &[u8]) -> Self {
        self.0.extend_from_slice(s);
        self
    }
    fn spaces(self, n: usize) -> Self {
        self.chars(&vec![b' '; n])
    }
}

/// Bar glyphs: `'('` is a full segment; `' '`..`'\''` are 0–7 eighths.
const BAR_FULL: u8 = b'(';

impl Game {
    fn print(&mut self, text: Text) {
        let assets = self.assets.clone();
        self.printer.print(&mut self.display, &assets.font, &assets.udg, &text.0);
    }

    /// Draws the panel frame from tiles. Bright cells in it are not
    /// recorded for restoring.
    pub fn draw_frame(&mut self) {
        self.restore_ptr = 0;
        self.draw_tile(0x91, 0, 1);
        let mut col = 4;
        for n in (1..=5).rev() {
            self.draw_tile(0x93, 0, col);
            self.draw_tile(0x93, 5, col);
            col += 4;
            if n == 4 || n == 3 {
                col += 1;
            }
        }
        self.draw_tile(0x92, 0, col + 2);
        for (i, tile) in (0x94..=0x97).enumerate() {
            self.draw_tile(tile, 0, 2 + 8 * i as u8);
        }
    }

    /// Adds pending points to the score and prints it.
    pub fn add_and_print_score(&mut self) {
        let mut carry = 0u8;
        for i in (0..6).rev() {
            let sum = self.status.score[i] + carry + self.status.pending[i];
            self.status.pending[i] = 0;
            self.status.score[i] = sum % 10;
            carry = sum / 10;
        }
        let digits = self.status.score.map(|d| b'0' + d);
        self.print(Text::default().at(2, 3).bright(1).ink(7).chars(&digits));
    }

    /// Redraws the score, lives, bars and inventory.
    pub fn draw_status(&mut self) {
        self.add_and_print_score();

        let lives = self.status.lives;
        self.print(
            Text::default()
                .at(3, 11)
                .ink(6)
                .chars(&[b'0' + lives / 10, b'0' + lives % 10])
                // Colour the bar rows, then print with ink 8 to keep it.
                .at(1, 16)
                .ink(2)
                .spaces(1)
                .ink(4)
                .spaces(3)
                .at(2, 16)
                .ink(7)
                .spaces(4)
                .at(3, 16)
                .ink(6)
                .spaces(4)
                .ink(8),
        );

        for row in (1..=3).rev() {
            let bar = &mut self.status.bars[row as usize - 1];
            if *bar >= 0x7F {
                *bar = 0x7F;
            }
            let v = *bar;
            let full = vec![BAR_FULL; ((v >> 5) & 3) as usize];
            let last = if v == 0x7F { BAR_FULL } else { b' ' + ((v >> 2) & 7) };
            self.print(Text::default().at(row, 16).chars(&full).chars(&[last]));
        }

        self.print(Text::default().at(1, 21).spaces(8).at(2, 21).spaces(8));

        for (i, (graphic, attr)) in self.status.inventory.into_iter().enumerate() {
            if attr != 0 {
                let g = self.assets.graphic32(graphic);
                self.draw_block2x2(&g, 1, 21 + 2 * i as u8, attr);
            }
        }
    }

    /// Reduces bar `index` by `amount` and redraws its end.
    pub fn reduce_bar(&mut self, index: usize, amount: u8) {
        let v = self.status.bars[index].saturating_sub(amount);
        self.status.bars[index] = v;
        let col = 16 + ((v >> 5) & 3);
        let w = if v < 4 { v + 3 } else { v };
        let glyph = b' ' + ((w >> 2) & 7);
        self.print(Text::default().ink(8).bright(1).at(index as u8 + 1, col).chars(&[glyph]));
    }
}
