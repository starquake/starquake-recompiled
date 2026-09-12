//! The screens around a game: the intro, and the framed message screens
//! the other flows are drawn on.

use crate::display::{ATTR_LEN, BITMAP_LEN};
use crate::game::Game;
use crate::host::Host;

/// Texts and tables in the original.
mod at {
    pub const INTRO_TEXT: usize = 0x6675;
    /// Corner tiles of a message screen: column, row, tile.
    pub const CORNERS: usize = 0x6661;
    /// Tune pointers (the tunes themselves are not played yet).
    pub const TUNES: usize = 0x65F4;
    pub const GAME_OVER_TEXT: usize = 0x6738;
    /// After the rooms-visited score, and after the score digits.
    pub const AFTER_VISITED: usize = 0x67D0;
    pub const AFTER_SCORE: usize = 0x67F0;
    pub const DOT: usize = 0x6828;
    pub const ZERO: usize = 0x6833;
    pub const AFTER_TIME: usize = 0x683B;
    pub const INITIALS_TEXT: usize = 0x6876;
    pub const HEROES_TEXT: usize = 0x6553;
    /// Line layout of the high-score table: "n." then the entry.
    pub const HEROES_LINE: usize = 0x658F;
}

/// Entries in the high-score table.
const HEROES: usize = 8;
const ENTRY: usize = 10;

const BORDER_TOP: u8 = 0x8A;
const BORDER_SIDE: u8 = 0x8B;

impl Game {
    /// Blanks the screen: black paper, white ink, black border.
    pub fn clear_screen(&mut self) {
        self.display.mem[..=BITMAP_LEN].fill(0);
        self.display.mem[BITMAP_LEN..BITMAP_LEN + ATTR_LEN].fill(7);
        self.display.border = 0;
    }

    /// A blank screen with the game's tile border, drawn in `colour`.
    pub fn message_screen(&mut self, colour: u8) {
        self.colour = colour;
        self.clear_screen();
        // The original drops the restore records made while a screen like
        // this is drawn, by pointing the list at ROM.
        self.restore_ptr &= 0x00FF;
        let mut col = 2;
        for _ in 0..7 {
            self.draw_tile(BORDER_TOP, 0, col);
            self.draw_tile(BORDER_TOP, 0x16, col);
            col += 4;
        }
        let mut row = 2;
        for _ in 0..5 {
            self.draw_tile(BORDER_SIDE, row, 0);
            self.draw_tile(BORDER_SIDE, row, 0x1E);
            row += 4;
        }
        self.draw_corners();
    }

    /// The four corner tiles of a message screen.
    pub(crate) fn draw_corners(&mut self) {
        let ram = self.assets.clone();
        for i in 0..4 {
            let e = at::CORNERS + i * 3;
            let (col, row, tile) = (ram.ram[e], ram.ram[e + 1], ram.ram[e + 2]);
            self.draw_tile(tile, row, col);
        }
    }

    /// Waits for every key to be released, then plays tune `tune` until it
    /// ends or a key is pressed.
    ///
    /// The original polls the keyboard between speaker toggles; here a key
    /// stops the tune at the next frame instead, which is as often as the
    /// host reports input.
    pub fn tune_until_key(&mut self, host: &mut dyn Host, tune: u8) {
        self.wait_keys_released(host);
        let entry = at::TUNES + tune as usize * 2;
        let ram = self.assets.clone();
        let addr = ram.ram[entry] as usize | (ram.ram[entry + 1] as usize) << 8;
        let (edges, total) = crate::music::tune(&ram.ram, addr);

        let mut next = 0;
        let mut t = 0;
        while t < total {
            let end = t + crate::sound::FRAME_T;
            let first = next;
            while next < edges.len() && edges[next].0 < end {
                next += 1;
            }
            // `sync` cleared this and kept its capacity; extending reuses it.
            self.music.extend(
                edges[first..next]
                    .iter()
                    .map(|&(at, level)| (at - t, level)),
            );
            self.sync(host);
            // The original's player scans a half-row of the keyboard between
            // speaker toggles and stops on any pressed bit, so two keys held
            // together end the tune just as one does. The release-wait above
            // keeps `key_code`, which is what the caller at 6600 uses.
            if crate::controls::any_key(&self.input) {
                break;
            }
            t = end;
        }
    }

    /// Shows the picture the tape painted while the game loaded, until a key
    /// is pressed. The picture is a block on the tape rather than anything
    /// the program draws, so this is an addition: on a Spectrum it simply sat
    /// there for the minutes the rest of the tape took to load.
    pub fn loading_screen(&mut self, host: &mut dyn Host) {
        let Some(screen) = self.assets.loading_screen.clone() else {
            return;
        };
        let n = screen.len().min(BITMAP_LEN + ATTR_LEN);
        self.display.mem[..n].copy_from_slice(&screen[..n]);
        self.display.border = 0;
        self.ask_key(host, |k| k != 0);
    }

    /// The report BLOB's flight computer gives on the way down.
    pub fn intro_screen(&mut self) {
        self.message_screen(4);
        self.print_text(at::INTRO_TEXT);
    }

    pub fn intro(&mut self, host: &mut dyn Host) {
        self.intro_screen();
        self.tune_until_key(host, 4);
        self.clear_screen();
    }

    /// Prints a number without leading zeros.
    fn print_number(&mut self, mut v: u16) {
        let mut started = false;
        for unit in [10000u16, 1000, 100, 10] {
            if !started && v < unit {
                continue;
            }
            started = true;
            let digit = (v / unit) as u8;
            v %= unit;
            self.print_bytes(&[b'0' + digit]);
        }
        self.print_bytes(&[b'0' + v as u8]);
    }

    /// Draws the nine core slots as a 3 × 3 grid: white for the pieces
    /// found, red for those still missing, which then flash.
    pub(crate) fn draw_core_grid(&mut self, row: u8, col: u8) {
        self.core_grid = (col, row);
        for r in 0..3 {
            for c in 0..3 {
                let piece = self.core_slots[r * 3 + c];
                let colour = if piece >= 0x80 { 2 } else { 7 };
                let g = self.assets.graphic32(piece & 0x7F);
                self.draw_block2x2(&g, row + 2 * r as u8, col + 2 * c as u8, colour);
            }
        }
        for _ in 0..13 {
            self.sparkle_core();
        }
    }

    /// Whether the score gets into the high-score table.
    fn beats_high_score(&self) -> bool {
        let last = &self.high_scores[(HEROES - 1) * ENTRY + 3..][..6];
        self.score_digits.as_slice() > last
    }

    /// Types three initials into the table and sorts it.
    fn enter_initials(&mut self, host: &mut dyn Host) {
        self.message_screen(7);
        self.print_text(at::INITIALS_TEXT);
        let mut initials = [b' '; 3];
        for slot in &mut initials {
            let k = self.ask_key(host, |k| k >= 0x20);
            *slot = k;
            self.print_bytes(&[k, b' ']);
            self.effects.push(7);
        }
        let last = (HEROES - 1) * ENTRY;
        self.high_scores[last..last + 3].copy_from_slice(&initials);
        self.high_scores[last + 3..last + 9].copy_from_slice(&self.score_digits);
        self.high_scores[last + 9] = self.adventure;
        // Bubble the new entry up the table.
        for i in (0..HEROES - 1).rev() {
            let (a, b) = (i * ENTRY, (i + 1) * ENTRY);
            if self.high_scores[a + 3..a + 9] < self.high_scores[b + 3..b + 9] {
                for k in 0..ENTRY {
                    self.high_scores.swap(a + k, b + k);
                }
            }
        }
    }

    /// The high-score table.
    fn core_of_heroes(&mut self, host: &mut dyn Host) {
        self.core_of_heroes_screen();
        self.tune_until_key(host, 2);
    }

    pub fn core_of_heroes_screen(&mut self) {
        self.message_screen(3);
        self.print_text(at::HEROES_TEXT);
        let layout: Vec<u8> = self.assets.ram[at::HEROES_LINE..at::HEROES_LINE + 17].to_vec();
        for i in 0..HEROES {
            let entry = &self.high_scores[i * ENTRY..(i + 1) * ENTRY];
            let mut line = layout.clone();
            line[1] = 6 + 2 * i as u8;
            line[3] = b'1' + i as u8;
            // Initials, a gap, then the score digits.
            line[5..8].copy_from_slice(&entry[0..3]);
            line[9..15].copy_from_slice(&entry[3..9]);
            let visited = entry[9];
            // Each line comes up in a new colour.
            self.random_ink();
            self.print_bytes(&line[..16]);
            self.print_number(visited as u16);
            self.print_bytes(b"/");
        }
    }

    /// The end of a game: the scores, then the high-score table.
    pub fn game_over(&mut self, host: &mut dyn Host) {
        self.game_over_screen();
        self.tune_until_key(host, 1);
        if self.beats_high_score() {
            self.enter_initials(host);
        }
        self.core_of_heroes(host);
    }

    /// What the end of a game shows: score, rooms visited, time taken, core
    /// pieces replaced, and the core itself.
    pub fn game_over_screen(&mut self) {
        self.message_screen(2);
        self.print_text(at::GAME_OVER_TEXT);
        let visited = self.adventure_score();
        self.adventure = visited;
        self.print_number(visited as u16);
        self.print_text(at::AFTER_VISITED);
        self.score_digits = self.status.score.map(|d| d + b'0');
        let digits = self.score_digits;
        self.print_bytes(&digits);
        self.print_text(at::AFTER_SCORE);

        let seconds = self.frames / crate::host::FRAMES_PER_SECOND;
        let (minutes, rest) = (seconds / 60, seconds % 60);
        self.print_number(minutes as u16);
        self.print_text(at::DOT);
        if rest < 10 {
            self.print_text(at::ZERO);
        }
        self.print_number(rest as u16);
        self.print_text(at::AFTER_TIME);

        let replaced = 9u8.saturating_sub(self.cores_left);
        if replaced < 10 {
            self.print_text(at::ZERO);
        }
        self.print_number(replaced as u16);
        self.draw_core_grid(0x0F, 0x15);
    }
}
