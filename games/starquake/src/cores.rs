//! The core room, and the ending.
//!
//! Room 199 holds the planet's core: nine slots, some of them empty at the
//! start of a game. Walking in with a piece that fits one of the holes puts
//! it in, and every second piece delivered counts towards the five that
//! finish the game.

use crate::display::{ATTR_LEN, BITMAP_LEN};
use crate::entities::{SLOTS, field};
use crate::game::Game;
use crate::host::Host;

mod at {
    /// "THE CORES COMPLETE...".
    pub const ENDING_TEXT: usize = 0x694D;
    /// Entity slot template; the core room resets more of it than a normal
    /// room entry does.
    pub const SLOT_TEMPLATE: usize = 0xA4A7;
}

/// The room the core is in.
pub const CORE_ROOM: u16 = 199;
/// Where the core grid is drawn while pieces are delivered, and in the
/// ending (row, column).
const GRID: (u8, u8) = (0x0C, 0x0D);
const ENDING_GRID: (u8, u8) = (0x0E, 0x0D);
const BLANK: u16 = 0xDF40;
/// Where BLOB is put down on the way out, and the graphic he leaves with.
const LEAVE_X: u8 = 0xF0;
const LEAVE_Y: u8 = 0x27;
const LEAVE_GRAPHIC: u16 = 0xE374;
/// Pieces to deliver before the game is finished.
const CORES_TO_FINISH: u8 = 5;

impl Game {
    /// XORs every attribute on the screen: a colour flash over everything.
    pub fn xor_attributes(&mut self, v: u8) {
        for a in &mut self.display.mem[BITMAP_LEN..BITMAP_LEN + ATTR_LEN] {
            *a ^= v;
        }
    }

    /// Draws a blank over one slot of the core grid, in `colour`.
    pub(crate) fn flash_core_slot(&mut self, slot: u8, colour: u8) {
        let (base_col, base_row) = self.core_grid;
        let row = base_row + 2 * (slot / 3);
        let col = base_col + 2 * (slot % 3);
        let blank = self.assets.graphic_at(BLANK);
        self.draw_block2x2(&blank, row, col, colour);
    }

    /// Twinkles one of the holes still left in the core.
    pub(crate) fn sparkle_core(&mut self) {
        self.rng.step();
        let slot = (self.rng.lo() & 0x3F) % 9;
        if self.core_slots[slot as usize] < 0x80 {
            return;
        }
        let colour = (self.rng.hi() & 0x3F) % 5 + 2;
        self.flash_core_slot(slot, colour);
    }

    /// Empties inventory slot `slot` and parks the item it held in the core
    /// room. A carried item is marked by its row, which is `2 + the slot`.
    fn take_item(&mut self, slot: usize) {
        self.status.inventory[slot] = (0, 0);
        let row = 2 + slot as u8;
        if let Some(item) = self.items.iter_mut().find(|i| i.row() == row) {
            item.0[1] = 0x0A;
            item.0[2] = CORE_ROOM as u8;
        }
    }

    /// The piece flies out of the panel and into its hole in the core.
    fn deliver(&mut self, slot: usize, index: usize) -> bool {
        self.status.pending[1] = 1;
        self.add_and_print_score();

        let col = 0x15 + 2 * slot as u8;
        let mut colour = 7u8;
        for _ in 0..0x19 {
            colour ^= 5;
            self.flash_core_slot(index as u8, colour);
            let blank = self.assets.graphic_at(BLANK);
            self.draw_block2x2(&blank, 1, col, colour);
            self.request_effect(3);
        }

        self.draw_core_grid(GRID.0, GRID.1);
        // The hole is filled with its own number, so it is no longer
        // missing (the original stores the slot, not the piece).
        self.core_slots[index] = index as u8;
        self.draw_core_grid(GRID.0, GRID.1);

        self.cores_left = self.cores_left.wrapping_sub(1);
        self.take_item(slot);
        self.draw_status();
        if !self.cores_left.is_multiple_of(2) {
            return false;
        }

        // Every second piece counts, and the screen flashes.
        self.cores += 1;
        for _ in 0..10 {
            for v in (0..8u8).rev() {
                self.xor_attributes(v);
                self.request_effect(0x11);
            }
        }
        self.cores == CORES_TO_FINISH
    }

    /// The core room: puts in every piece carried that fits. Returns whether
    /// that finished the game.
    ///
    /// # Panics
    ///
    /// If the loaded game data is too short to hold what the original keeps
    /// there, which means the file was not Starquake.
    pub fn core_room(&mut self, host: &mut dyn Host) -> bool {
        let template: [u8; 9] = self.assets.ram[at::SLOT_TEMPLATE..at::SLOT_TEMPLATE + 9]
            .try_into()
            .expect("slot template");
        for slot in 0..SLOTS {
            self.entities[slot].0[..9].copy_from_slice(&template);
        }
        self.draw_core_grid(GRID.0, GRID.1);

        // The inventory is gone over twice, so a piece behind one that has
        // just gone in is seen too.
        for _ in 0..2 {
            for slot in 0..4 {
                if self.status.inventory[slot].1 == 0 {
                    continue;
                }
                for index in 0..9 {
                    // As in the original, the slot is re-read each time: a
                    // slot emptied above matches a hole numbered zero.
                    if self.core_slots[index].wrapping_sub(0x80) != self.status.inventory[slot].0 {
                        continue;
                    }
                    if self.deliver(slot, index) {
                        self.ending(host);
                        return true;
                    }
                }
            }
        }

        // The core's guardians dance about for a while.
        self.spawn_enemies();
        for _ in 0..0xC8 {
            self.frame_with(host);
            self.update_enemies();
            self.sparkle_core();
            self.sparkle_core();
            let id = (self.rng.lo() & 1) + 0x14;
            self.request_effect(id);
        }

        let b = &mut self.entities[0].0;
        b[field::X] = LEAVE_X;
        b[field::Y] = LEAVE_Y;
        b[field::GRAPHIC..field::GRAPHIC + 2].copy_from_slice(&LEAVE_GRAPHIC.to_le_bytes());
        self.room -= 1;
        self.entry_reason = 0;
        false
    }

    /// Draws the sign-off: the finished core, and what BLOB makes of it.
    pub fn ending_screen(&mut self) {
        self.message_screen(2);
        self.draw_core_grid(ENDING_GRID.0, ENDING_GRID.1);
        self.print_text(at::ENDING_TEXT);
    }

    /// The end of the game: the last of the scoring, the sign-off, and a
    /// tune. The caller goes on to the game-over screen.
    fn ending(&mut self, host: &mut dyn Host) {
        self.final_scoring();
        self.ending_screen();
        self.tune_until_key(host, 5);
    }
}
