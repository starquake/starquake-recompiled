//! Starting a new game: controls, starting values, and scattering the core
//! pieces and other items over the planet.

use crate::entities::SLOTS;
use crate::game::Game;
use crate::pickups::RoomSet;

/// Tables in the original.
mod table {
    /// Key names per control method, 5 per method (method 5 is the
    /// player-defined set, kept in [`crate::game::Game::udk`]).
    pub const METHOD_KEYS: usize = 0x5E52;
    /// (offset, value) pairs for the variables starting at `D2BE`.
    pub const START_VALUES: usize = 0x6343;
    /// Candidate rooms for the two special items (4 each).
    pub const SPECIAL_ROOMS: usize = 0x5E50;
    /// Room pairs for the other scattered items.
    pub const ITEM_ROOMS: usize = 0x5E2C;
    /// Entity slot template.
    pub const SLOT_TEMPLATE: usize = 0xA4A7;
}

const KEMPSTON: u8 = 1;
const PLAYER_DEFINED: u8 = 5;
const START_X: u8 = 0x88;
const START_Y: u8 = 0x3F;
const START_GRAPHIC: u16 = 0xE734;

impl Game {
    /// Sets item `k` to lie (not yet placed) in the room given by a room
    /// table byte (the room number halved).
    fn scatter_item(&mut self, k: usize, room_byte: u8, graphic: u8) {
        let item = &mut self.items[k].0;
        item[1] = room_byte & 0x80;
        item[2] = room_byte << 1;
        item[3] = graphic;
    }

    fn set_start_value(&mut self, offset: u8, value: u8) {
        match offset {
            0x0A => self.room = (self.room & 0xFF00) | value as u16,
            0x0E => self.status.lives = value,
            0x0F..=0x11 => self.status.bars[offset as usize - 0x0F] = value,
            0x29 => self.cores_left = value,
            _ => panic!("unexpected start value offset {offset:#04x}"),
        }
    }

    /// Resets everything for a new game with control method `method`
    /// (1 Kempston, 2 cursor, 3 Sinclair, 4 keyboard, 5 player-defined).
    pub fn new_game(&mut self, method: u8) {
        let ram = self.assets.clone();
        let ram = &ram.ram;

        self.controls.kempston = method == KEMPSTON;
        if method != KEMPSTON {
            let names: [u8; 5] = if method == PLAYER_DEFINED {
                self.udk
            } else {
                let at = table::METHOD_KEYS + method as usize * 5;
                ram[at..at + 5].try_into().unwrap()
            };
            let pause = self.udk_pause;
            self.controls.set_keys(ram, names, pause);
        }

        // Game variables: zeroed, then the starting values.
        self.pickups_in_room = 0;
        self.var_d2bf = 0;
        self.bonus = Default::default();
        self.entry_reason = 0;
        self.saved_state = 0;
        self.seed = 0;
        self.room = 0;
        self.objects.kind12 = None;
        self.status.lives = 0;
        self.status.bars = [0; 3];
        self.status.incoming = (0, 0);
        self.status.inventory = [(0, 0); 4];
        self.status.outgoing = (0, 0);
        self.saved_position = (0, 0);
        self.core_slots = [0; 9];
        self.cores_left = 0;
        self.cores = 0;
        self.var_d2e9 = 0;
        self.var_d2ea = 0;
        let mut t = table::START_VALUES;
        for offset in 0..45 {
            if ram[t] == offset {
                self.set_start_value(offset, ram[t + 1]);
                t += 2;
            }
        }

        self.seed = self.frames as u16;
        self.rng.a = self.frames as u16;

        // The two special items, each in one of four rooms.
        for i in 0..2 {
            self.rng.step();
            let room = ram[table::SPECIAL_ROOMS + i * 4 + (self.rng.lo() & 3) as usize];
            self.scatter_item(i, room, 0x0F + i as u8);
        }

        // Five core pieces, in random slots of the nine.
        for _ in 0..5 {
            let piece = loop {
                self.rng.step();
                // (The original subtracts 15 until negative, then adds
                // 0x98: the remainder plus 0x89.)
                let p = self.rng.lo() % 15 + 0x89;
                let p = if p >= 0x8F { p.wrapping_add(0x0B) } else { p };
                if !self.core_slots.contains(&p) {
                    break p;
                }
            };
            loop {
                self.rng.step();
                let slot = (self.rng.lo() % 9) as usize;
                if self.core_slots[slot] == 0 {
                    self.core_slots[slot] = piece;
                    break;
                }
            }
        }
        for (i, slot) in self.core_slots.iter_mut().enumerate() {
            if *slot == 0 {
                *slot = 0x80 + i as u8;
            }
        }

        // Eighteen more items, each in one of a pair of rooms.
        let mut t = table::ITEM_ROOMS;
        let mut k = 2;
        for group in 0..2 {
            self.rng.step();
            let mut c = self.rng.lo() & 7;
            for _ in 0..9 {
                c += 1;
                if c >= 9 {
                    c = 0;
                }
                let mut g = self.core_slots[c as usize];
                if group == 1 && self.rng.hi() >= 0x96 {
                    g = (g & 7) + 0x1A;
                }
                self.rng.step();
                let alternative = (self.rng.lo() >= 0x7F) as usize;
                self.scatter_item(k, ram[t + alternative], g & 0x7F);
                k += 1;
                t += 2;
            }
        }

        self.bonus_rooms = RoomSet([0xFF; 64]);
        self.unvisited_rooms = RoomSet([0xFF; 64]);
        for tp in &mut self.teleporters {
            tp.1 |= 1;
        }
        self.status.score = [0; 6];
        self.status.pending = [0; 6];
        let template: [u8; 9] = ram[table::SLOT_TEMPLATE..table::SLOT_TEMPLATE + 9].try_into().unwrap();
        for slot in 0..SLOTS {
            self.entities[slot].0 = [0; 32];
            self.entities[slot].0[..9].copy_from_slice(&template);
        }
        let guard = crate::display::BITMAP_LEN + crate::display::ATTR_LEN;
        self.display.mem[guard..guard + 32].fill(0x40);

        let b = &mut self.entities[0].0;
        b[5] = START_X;
        b[6] = START_Y;
        b[7..9].copy_from_slice(&START_GRAPHIC.to_le_bytes());
        b[0x0A] = 0;
        b[0x13] = 1;
        self.frames = 0;
    }

    /// Rooms visited, as the "adventure score" shown at the end (visits × 50
    /// / 256). Like the original, leaves the room number at 512.
    pub fn adventure_score(&mut self) -> u8 {
        let visited = (0..512u16).filter(|&r| !self.unvisited_rooms.contains(r)).count() as u16;
        self.room = 512;
        (visited.wrapping_mul(50) >> 8) as u8
    }

    /// End-of-game scoring: a thousand points, then the last three digits
    /// are drawn from a generator seeded with the score and adventure score.
    pub fn final_scoring(&mut self) {
        self.status.pending[2] = 1;
        self.draw_status();
        let adventure = self.adventure_score();
        let s = self.status.score;
        self.rng.a = u16::from_le_bytes([s[0], s[1]]);
        self.rng.b = u16::from_le_bytes([s[2], adventure]);
        self.rng.c = u16::from_le_bytes([adventure, adventure]);
        self.rng.b_countdown = 3;
        self.rng.c_countdown = 3;
        for _ in 0..30 {
            self.rng.step();
        }
        for i in 3..5 {
            self.rng.step();
            self.status.score[i] = self.rng.lo() % 10;
        }
        self.status.score[5] = if self.rng.hi() & 1 != 0 { 5 } else { 0 };
        self.draw_status();
    }
}
