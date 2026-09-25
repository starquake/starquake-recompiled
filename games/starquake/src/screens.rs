//! Screens that take over the room: security doors, teleporter booths and
//! the Cheops pyramid. Each clears the room area, runs, and returns the
//! reason to re-enter the room with.

use crate::blob::Modal;
use crate::entities::field;
use crate::entry::reason;
use crate::game::Game;
use crate::host::Host;

/// Where things are in the original.
mod at {
    /// Texts (0xFF-terminated, with print control codes).
    pub const SECURITY_TEXT: usize = 0xCBF3;
    pub const CHEOPS_TITLE: usize = 0xCE04;
    pub const CHEOPS_KEY_CODE: usize = 0xCD02;
    pub const EXCHANGE_FOR: usize = 0xCD77;
    pub const HIT_1_TO_5: usize = 0xCD90;
    pub const TELEPORT_ENTERED: usize = 0xCEE0;
    pub const TELEPORT_ENTER_CODE: usize = 0xCF49;
    pub const TELEPORT_DASHES: usize = 0xCF76;
    pub const TELEPORTING: usize = 0xCFDB;
    pub const NOT_RECOGNISED: usize = 0xD010;
    pub const AUTHORISED: usize = 0xD744;
    pub const INVALID: usize = 0xD76A;
    /// Teleporter names: 15 × (5 letters, room number).
    pub const TELEPORTERS: usize = 0xD036;
    pub const PYRAMID_GRAPHIC: u16 = 0x93A8;
}

const BLANK: u16 = 0xDF40;
const MASTER_KEY: u8 = 0x0F;
const WILDCARD: u8 = 0x0E;

/// Where a security door's screen draws its code, which the code is made
/// from along with the game's seed and the room.
const DOOR_CODE_AT: (u8, u8) = (0x11, 0x0B);

/// The key code cards a code asks for, by graphic (9 to 13), made from the
/// game's `seed`, the `room` and where the screen draws the code
/// (`row`, `col`). A door's screen asks for all three, a pyramid's for the
/// first two.
pub fn code_items(seed: u16, room: u16, row: u8, col: u8) -> [u8; 3] {
    let [seed_lo, seed_hi] = seed.to_le_bytes();
    let first = row ^ seed_hi ^ room as u8;
    let second = first ^ seed_lo ^ col;
    let third = second ^ seed_hi ^ row;
    [first, second, third].map(|a| (a & 0x3F) % 5 + 9)
}

/// The inventory slot that answers a code's card `wanted`, among `slots`
/// (the carried items' graphics) not yet `used` (a bit a slot, 8 for the
/// first): the access card, which answers any and stays, or that card, or
/// failing both a "?" card. Says whether the slot is used up.
pub fn match_slot(wanted: u8, slots: [u8; 4], used: u8) -> Option<(usize, bool)> {
    let free = |s: usize| used & (8 >> s) == 0;
    (0..4)
        .find_map(|s| match slots[s] {
            _ if !free(s) => None,
            MASTER_KEY => Some((s, false)),
            g if g == wanted => Some((s, true)),
            _ => None,
        })
        .or_else(|| {
            (0..4)
                .find(|&s| free(s) && slots[s] == WILDCARD)
                .map(|s| (s, true))
        })
}

/// Which of a code's cards `wanted` the carried items `slots` answer, as a
/// door's screen checks them in turn.
pub fn answered(wanted: &[u8], slots: [u8; 4]) -> Vec<bool> {
    let mut used = 0u8;
    wanted
        .iter()
        .map(|&w| {
            let hit = match_slot(w, slots, used);
            if let Some((slot, true)) = hit {
                used |= 8 >> slot;
            }
            hit.is_some()
        })
        .collect()
}

impl Game {
    /// The key code cards the door in `room` asks for this game (#94).
    pub fn door_code(&self, room: u16) -> [u8; 3] {
        code_items(self.seed, room, DOOR_CODE_AT.0, DOOR_CODE_AT.1)
    }
}

impl Game {
    /// Prints an original text from its address.
    pub(crate) fn print_text(&mut self, addr: usize) {
        let assets = self.assets.clone();
        let end = assets.ram[addr..].iter().position(|&b| b == 0xFF).unwrap();
        let udg = if self.title_udg {
            &assets.title_udg
        } else {
            &assets.udg
        };
        self.work.characters += self.printer.print(
            &mut self.display,
            &assets.font,
            udg,
            &assets.ram[addr..addr + end],
        );
    }

    pub(crate) fn print_bytes(&mut self, bytes: &[u8]) {
        let assets = self.assets.clone();
        let udg = if self.title_udg {
            &assets.title_udg
        } else {
            &assets.udg
        };
        self.work.characters += self
            .printer
            .print(&mut self.display, &assets.font, udg, bytes);
    }

    /// Switches the print ink to a new random colour.
    pub fn random_ink(&mut self) {
        self.rng.step();
        let mut ink = (self.rng.hi() & 0x3F) % 6 + 2;
        if ink == self.screen_ink {
            ink ^= 1;
        }
        self.screen_ink = ink;
        self.print_bytes(&[0x10, ink]);
    }

    /// Alternates the print ink, for flashing messages.
    fn flash_ink(&mut self) {
        self.flash_phase ^= 1;
        let ink = if self.flash_phase != 0 {
            (self.screen_ink ^ 7) | 2
        } else {
            self.screen_ink
        };
        self.print_bytes(&[0x10, ink]);
    }

    fn random_colour(&mut self) -> u8 {
        self.rng.lo() % 6 + 2
    }

    /// Draws the code items of a code check.
    fn draw_code(&mut self) {
        let (col, row) = (self.code_pos.0, self.code_pos.1);
        for i in 0..self.code_len as usize {
            let (graphic, attr) = (self.code[i * 2], self.code[i * 2 + 1]);
            let g = self.assets.graphic32(graphic);
            self.draw_block2x2(&g, row, col.wrapping_add(4 * i as u8), attr);
        }
    }

    /// The code check behind security doors and the pyramid: `count` code
    /// items, derived from the game seed, the room and the position, must
    /// be matched by items carried. Returns whether access was granted.
    fn code_check(&mut self, host: &mut dyn Host, count: u8, row: u8, col: u8) -> bool {
        self.code_len = count;
        self.code_pos = (col, row);
        self.code = [3; 6];
        self.pause_frames(host, 15);
        for (i, card) in code_items(self.seed, self.room, row, col)
            .into_iter()
            .enumerate()
        {
            self.code[i * 2] = card;
        }
        self.draw_code();

        let mut used = 0u8;
        for item in 0..count as usize {
            // Scanning: a code item flashes in random colours.
            for _ in 0..25 {
                self.random_ink();
                let k = (self.rng.lo() & 0x1F) % count;
                let blank = self.assets.graphic_at(BLANK);
                self.draw_block2x2(&blank, row, col.wrapping_add(k * 4), self.screen_ink);
                self.request_effect((self.rng.hi() & 3) + 0x0C);
            }
            self.draw_code();
            self.draw_code();

            let wanted = self.code[item * 2];
            let slots = self.status.inventory.map(|(g, _)| g);
            if let Some((slot, consume)) = match_slot(wanted, slots, used) {
                if consume {
                    used |= 8 >> slot;
                }
                self.random_ink();
                let mut ink = self.screen_ink;
                let slot_col = 0x15 + 2 * slot as u8;
                let item_col = col.wrapping_add(4 * item as u8);
                let blank = self.assets.graphic_at(BLANK);
                for _ in 0..10 {
                    ink = (ink ^ 7) | 2;
                    self.draw_block2x2(&blank, 1, slot_col, ink);
                    self.draw_block2x2(&blank, row, item_col, ink);
                    self.request_effect(3);
                }
                self.code[item * 2 + 1] = 7;
            }
            self.draw_status();
        }

        self.pause_frames(host, 20);
        let granted = (0..count as usize).all(|i| self.code[i * 2 + 1] == 7);
        let (times, text) = if granted {
            (35, at::AUTHORISED)
        } else {
            (40, at::INVALID)
        };
        for _ in 0..times {
            self.flash_ink();
            self.print_text(text);
            self.request_effect(0x0F);
        }
        granted
    }

    /// Stepping back out of a screen: BLOB is snapped to the grid.
    fn leave_screen(&mut self) -> u8 {
        let y = self.entities[0].0[field::Y];
        self.entities[0].0[field::Y] = (y.wrapping_add(1) & 0xF8).wrapping_sub(1);
        reason::KEEP_ENEMIES
    }

    fn security_door(&mut self, host: &mut dyn Host) -> u8 {
        self.display.clear_room_area();
        self.random_ink();
        self.print_text(at::SECURITY_TEXT);
        self.draw_tile(0x25, 0x0A, 0x0C);
        self.draw_tile(0x26, 0x0A, 0x10);
        self.request_effect(8);
        let cards = self.door_code(self.room);
        if !self.doors_seen.iter().any(|d| d.room == self.room) {
            self.doors_seen.push(crate::game::SeenDoor {
                room: self.room,
                cards,
            });
        }
        if self.code_check(host, 3, DOOR_CODE_AT.0, DOOR_CODE_AT.1) {
            self.request_effect(0x0A);
            let b = &mut self.entities[0].0;
            b[field::X] = if b[crate::blob::b::INPUT] & 1 != 0 {
                b[field::X].wrapping_add(0x30)
            } else {
                b[field::X].wrapping_sub(0x30)
            };
        }
        self.leave_screen()
    }

    fn pyramid_title(&mut self) {
        self.random_ink();
        self.print_text(at::CHEOPS_TITLE);
        let attr = self.random_colour();
        let g = self.assets.graphic_at(at::PYRAMID_GRAPHIC);
        self.draw_block2x2(&g, 8, 0x16, attr);
        self.random_ink();
    }

    /// Draws choice `i` of the pyramid trade (0–3 offers, 4 your own item).
    fn draw_offer(&mut self, i: u8) {
        self.random_ink();
        let graphic = self.offers[i as usize];
        let col = i * 6 + 4;
        let g = self.assets.graphic32(graphic);
        let attr = self.random_colour();
        self.draw_block2x2(&g, 0x10, col, attr);
        self.print_bytes(&[0x16, 0x0F, col - 2, b'1' + i, b'.']);
    }

    fn cheops(&mut self, host: &mut dyn Host) -> u8 {
        self.display.clear_room_area();
        self.pyramid_title();
        self.print_text(at::CHEOPS_KEY_CODE);
        self.request_effect(0x0B);
        if !self.code_check(host, 2, 0x0F, 0x0D) {
            return self.leave_screen();
        }
        self.display.clear_room_area();
        self.pyramid_title();

        // The item to trade: the first carried non-core item, else the last
        // one carried.
        let slots = self.status.inventory;
        let slot = (0..4)
            .find(|&s| slots[s].0 != 0 && !(9..0x1A).contains(&slots[s].0))
            .or_else(|| (0..4).rev().find(|&s| slots[s].0 != 0))
            .unwrap_or(0);
        self.offers[4] = slots[slot].0;
        // Four missing core pieces on offer.
        for i in (0..4).rev() {
            self.offers[i] = loop {
                self.rng.step();
                let c = self.core_slots[(self.rng.lo() % 9) as usize];
                if c >= 0x80 {
                    break c & 0x3F;
                }
            };
        }
        self.print_text(at::EXCHANGE_FOR);
        self.random_ink();
        self.print_text(at::HIT_1_TO_5);
        let g = self.assets.graphic32(self.offers[4]);
        let attr = self.random_colour();
        self.draw_block2x2(&g, 0x0C, 0x11, attr);
        for i in 0..5 {
            self.draw_offer(i);
        }
        let choice = self.wait_key(host, |k| (b'1'..=b'5').contains(&k)) - b'1';
        for _ in 0..35 {
            self.request_effect(0x10);
            self.draw_offer(choice);
        }
        self.status.inventory[slot].0 = self.offers[choice as usize];
        self.draw_status();
        self.bonus_rooms.set(self.room, false);
        self.leave_screen()
    }

    /// Every teleporter on the planet, its room and code, from the
    /// original's table: fifteen entries of five letters and a room (#95).
    ///
    /// # Panics
    ///
    /// If the loaded game data is too short to hold what the original keeps
    /// there, which means the file was not Starquake.
    pub fn all_teleporters(&self) -> Vec<crate::game::SeenTeleporter> {
        let ram = &self.assets.ram;
        (0..15)
            .map(|j| {
                let e = at::TELEPORTERS + j * 7;
                crate::game::SeenTeleporter {
                    code: ram[e..e + 5].try_into().unwrap(),
                    room: ram[e + 5] as u16 | (ram[e + 6] as u16) << 8,
                }
            })
            .collect()
    }

    /// The name of the teleporter in `room`, from the original's table.
    fn teleporter_name(&self, room: u16) -> [u8; 5] {
        let ram = &self.assets.ram;
        let entry = (0..15)
            .find(|&j| {
                let r = at::TELEPORTERS + j * 7 + 5;
                ram[r] as u16 | (ram[r + 1] as u16) << 8 == room
            })
            .unwrap_or(15);
        let start = (at::TELEPORTERS + entry * 7 + 5)
            .wrapping_add(1)
            .wrapping_sub(6)
            - if entry == 15 { 1 } else { 0 };
        ram[start..start + 5].try_into().unwrap()
    }

    fn teleport_booth(&mut self, host: &mut dyn Host) -> u8 {
        self.display.clear_room_area();
        self.restore_ptr = 0;
        self.random_ink();
        self.print_text(at::TELEPORT_ENTERED);
        self.draw_tile(0x24, 0x09, 0x17);
        let name = self.teleporter_name(self.room);
        if !self.teleporters_seen.iter().any(|t| t.code == name) {
            self.teleporters_seen.push(crate::game::SeenTeleporter {
                room: self.room,
                code: name,
            });
        }
        self.random_ink();
        self.print_bytes(&name);
        self.random_ink();
        self.print_text(at::TELEPORT_ENTER_CODE);
        self.random_ink();
        self.print_text(at::TELEPORT_DASHES);
        self.request_effect(7);
        self.random_ink();
        self.booth = true;
        for i in 0..5 {
            let k = self.ask_key(host, |k| k >= 0x0A);
            self.typed_code[i] = k;
            self.print_bytes(&[k, b' ']);
            self.request_effect(0x11);
        }
        self.booth = false;
        let ram = self.assets.clone();
        let ram = &ram.ram;
        for j in 0..15 {
            let at = at::TELEPORTERS + j * 7;
            if ram[at..at + 5] == self.typed_code {
                self.room = ram[at + 5] as u16 | (ram[at + 6] as u16) << 8;
                for _ in 0..20 {
                    self.flash_ink();
                    self.print_text(at::TELEPORTING);
                    self.request_effect(0x10);
                }
                self.request_effect(9);
                return reason::TELEPORT;
            }
        }
        for _ in 0..40 {
            self.flash_ink();
            self.print_text(at::NOT_RECOGNISED);
            self.request_effect(0x0F);
        }
        self.leave_screen()
    }

    /// Runs a screen; returns the reason to re-enter the room with.
    pub fn run_modal(&mut self, modal: Modal, host: &mut dyn Host) -> u8 {
        match modal {
            Modal::SecurityDoor => self.security_door(host),
            Modal::Cheops => self.cheops(host),
            Modal::TeleportBooth => self.teleport_booth(host),
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn each_card_answers_one_slot_and_the_access_card_every_one() {
        let (two, four, eight) = (11, 12, 13);
        let none = [0; 4];
        assert_eq!(answered(&[two, four, eight], none), [false; 3]);
        assert_eq!(
            answered(&[two, four, eight], [four, 0, 0, 0]),
            [false, true, false]
        );
        assert_eq!(
            answered(&[two, two, eight], [two, 0, 0, 0]),
            [true, false, false],
            "one card answers one slot"
        );
        assert_eq!(
            answered(&[two, four, eight], [MASTER_KEY, 0, 0, 0]),
            [true; 3]
        );
        assert_eq!(
            answered(&[two, four, eight], [WILDCARD, four, 0, 0]),
            [true, true, false],
            "the ? card stands in for the first card nothing else answers"
        );
    }

    #[test]
    fn a_code_is_made_of_numbered_cards() {
        for seed in [0, 0x1234, 0xFFFF] {
            for room in [176, 210, 429] {
                let cards = code_items(seed, room, DOOR_CODE_AT.0, DOOR_CODE_AT.1);
                assert!(cards.iter().all(|c| (9..=13).contains(c)), "{cards:?}");
            }
        }
    }
}
