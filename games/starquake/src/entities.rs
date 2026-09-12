//! Entities (BLOB, enemies, specials, the shot) and enemy spawning.
//!
//! There are six 32-byte entity slots: 0 is BLOB, 1–4 enemies (slot 4 is
//! taken by a special entity in some rooms), 5 BLOB's shot. Slots are kept
//! in the original's byte layout; the fields used so far are named below.

use crate::game::Game;

pub const SLOTS: usize = 6;

/// Byte offsets within a slot.
pub mod field {
    /// Pixels from the left.
    pub const X: usize = 0x05;
    /// Pixels from the bottom of the screen.
    pub const Y: usize = 0x06;
    /// Current graphic (2 bytes).
    pub const GRAPHIC: usize = 0x07;
    pub const COLOUR: usize = 0x09;
    /// Enemy: where it entered the room.
    pub const START_X: usize = 0x0A;
    pub const START_Y: usize = 0x0B;
    /// Enemy: direction bits (1 right, 2 left, 4 up, 8 down).
    pub const DIRECTION: usize = 0x0D;
    /// Enemy: first frame of its graphic set (2 bytes).
    pub const GRAPHIC_BASE: usize = 0x0E;
    pub const SPEED_X: usize = 0x11;
    pub const SPEED_Y: usize = 0x12;
    pub const ANIM_PERIOD: usize = 0x13;
    pub const ANIM_COUNT: usize = 0x14;
    pub const STATE: usize = 0x15;
    pub const STATE_COUNT: usize = 0x16;
    pub const TURN_PERIOD: usize = 0x17;
    pub const TURN_COUNT: usize = 0x18;
    pub const BEHAVIOUR: usize = 0x19;
}

/// BLOB-specific fields in slot 0.
pub mod blob {
    pub const COLOUR: usize = 0x09;
    pub const STATE: usize = 0x0A;
    pub const FALL: usize = 0x11;
    pub const PLATFORM_HELD: usize = 0x14;
}

#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub struct Entity(pub [u8; 32]);

impl Entity {
    pub fn x(&self) -> u8 {
        self.0[field::X]
    }
    pub fn y(&self) -> u8 {
        self.0[field::Y]
    }
    fn set_word(&mut self, at: usize, v: u16) {
        self.0[at..at + 2].copy_from_slice(&v.to_le_bytes());
    }
}

/// Bytes of a slot (from `X` on) saved in the enemy cache.
pub const CACHED: std::ops::Range<usize> = 0x05..0x1A;

/// Enemy bookkeeping across rooms.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub struct Spawner {
    /// Frames during which going back restores the previous room's enemies.
    pub timer: u8,
    pub last_room: u16,
    /// Enemy slots in use in the current room (3 when slot 4 is special).
    pub count: u8,
    pub room_before: u16,
    pub count_before: u8,
    /// Per-room random choices: which parameter bits are re-rolled per
    /// enemy, and the parameters.
    pub masks: [u8; 4],
    pub params: [u8; 4],
    /// High byte of the RNG when the room's enemies were rolled.
    pub seed: u8,
    /// Placement attempts for the last enemy.
    pub tries: u8,
}

const GRAPHIC_SETS: u16 = 0xB208;
const STATIONARY_GRAPHIC: u16 = 0xB2C8;
const SPECIAL_GRAPHIC: u16 = 0xAFC8;
const CORE_ROOM: u16 = 199;
const CACHE_RESET_ROOM: u16 = 198;
/// Tables in the original used by the core room.
const CORE_POSITIONS: usize = 0x9FB2;
const CORE_TEMPLATE: usize = 0x9FC0;

impl Game {
    /// Whether the 2 × 2 cells at pixel (x, y) are free: the first cell
    /// bright with paper 0–3, the others bright.
    fn cell_free(&self, x: u8, y: u8) -> bool {
        let i = (((0xBF - y) & 0xF8) as usize) << 2 | (x >> 3) as usize;
        let attr = |i: usize| self.display.mem[crate::display::BITMAP_LEN + i];
        attr(i) & 0x60 == 0x40
            && attr(i + 1) & 0x40 != 0
            && attr(i + 33) & 0x40 != 0
            && attr(i + 32) & 0x40 != 0
    }

    /// Picks a free place on a room edge for an enemy to come in from.
    fn place_enemy(&mut self, slot: usize) {
        self.spawner.tries = 0;
        loop {
            self.spawner.tries = self.spawner.tries.wrapping_add(1);
            if self.spawner.tries == 100 {
                self.entities[slot].0[field::Y] = 0;
                return;
            }
            self.rng.step();
            let lo = self.rng.lo();
            let (x, y) = if lo & 1 == 0 {
                // Top or bottom edge.
                let x = ((lo >> 1) % 23 + 4) << 3;
                let y = if self.rng.hi() & 1 != 0 { 0x11 } else { 0x8D };
                (x, y)
            } else {
                // Left or right edge.
                let y = ((lo.rotate_right(1) % 9 + 6) << 3) - 1;
                let x = if self.rng.hi() & 0x80 != 0 { 2 } else { 0xEE };
                (x, y)
            };
            if self.cell_free(x, y) {
                self.entities[slot].0[field::START_X] = x;
                self.entities[slot].0[field::START_Y] = y;
                return;
            }
        }
    }

    /// Rolls a new enemy into `slot` from the room's parameters.
    pub(crate) fn roll_enemy(&mut self, slot: usize) {
        for i in 0..4 {
            self.rng.step();
            self.spawner.params[i] ^= self.rng.lo() & self.spawner.masks[i];
        }
        let [p0, p1, p2, p3] = self.spawner.params;
        let masks = self.spawner.masks;

        let kind = if masks[0] & 0x1F != 0 {
            self.rng.hi() % 15 + 2
        } else {
            p0 & 0x1F
        };
        let colour = if masks[0] & 0x80 != 0 {
            self.rng.step();
            self.rng.lo() % 5 + 2
        } else {
            p0.rotate_left(3)
        };
        let speed = p2 & 0x0F;

        let e = &mut self.entities[slot];
        e.set_word(field::GRAPHIC_BASE, (kind as u16).wrapping_mul(0xC0).wrapping_add(GRAPHIC_SETS));
        e.0[field::COLOUR] = colour & 7;
        e.0[field::ANIM_COUNT] = p1;
        e.0[field::ANIM_PERIOD] = (p2 >> 4) % 5 + 4;
        e.0[field::TURN_PERIOD] = if speed == 0 { 0x64 } else { 1 << (speed % 5 + 1) };
        e.0[field::TURN_COUNT] = 8;
        e.0[field::BEHAVIOUR] = if kind == 2 { 5 } else { (p3 & 0x0F) % 5 };
        e.0[field::DIRECTION] = 0x55u8.rotate_right(p3 as u32);
        e.0[field::X] = 0;
        e.0[field::Y] = 0x0F;
        e.set_word(field::GRAPHIC, 0xDF40);
        e.0[field::SPEED_X] = 2;
        e.0[field::SPEED_Y] = 2;
        e.0[field::STATE] = 0;
        e.0[field::STATE_COUNT] = 0;
        self.place_enemy(slot);
    }

    fn place_special(&mut self, x: u8, y: u8) {
        let e = &mut self.entities[4];
        e.0[field::X] = x;
        e.0[field::Y] = y;
        e.set_word(field::GRAPHIC, SPECIAL_GRAPHIC);
        e.0[field::COLOUR] = 7;
        self.spawner.count = 3;
    }

    /// Stationary entities at tile type 8, and the special entity.
    fn spawn_fixed(&mut self) {
        let points = self.objects.type8.clone();
        for (i, &(col, row)) in points.iter().enumerate() {
            let e = &mut self.entities[points.len() - i];
            e.0[field::X] = col.rotate_left(3);
            e.0[field::Y] = 0x18u8.wrapping_sub(row).rotate_left(3).wrapping_sub(1);
            e.set_word(field::GRAPHIC, STATIONARY_GRAPHIC);
            e.0[field::DIRECTION] = 1;
            e.0[field::ANIM_PERIOD] |= 8;
            e.0[field::ANIM_COUNT] = 1;
            e.0[field::STATE] = 1;
            e.0[field::BEHAVIOUR] = 6;
        }
        if let Some((x, y)) = self.objects.kind12 {
            self.place_special(x, y.wrapping_sub(8));
        }
        let blob = self.entities[0];
        if blob.0[blob::STATE] == 2 {
            self.place_special(blob.x(), blob.y().wrapping_sub(8));
        }
    }

    fn spawn_core_room(&mut self) {
        self.spawner.count = 4;
        for slot in 1..=4 {
            self.entities[slot].0[field::X..field::X + 5].copy_from_slice(&[0, 0, 0x40, 0xDF, 0]);
        }
        self.rng.step();
        let mut pos = CORE_POSITIONS + (self.rng.lo() & 6) as usize;
        let template = &self.assets.ram[CORE_TEMPLATE..CORE_TEMPLATE + 19];
        for slot in 1..=self.cores as usize {
            let e = &mut self.entities[slot];
            e.0[field::X] = self.assets.ram[pos];
            e.0[field::Y] = self.assets.ram[pos + 1];
            e.0[field::GRAPHIC..field::GRAPHIC + 19].copy_from_slice(template);
            pos += 2;
        }
    }

    /// Fills the enemy slots for the room just entered.
    pub fn spawn_enemies(&mut self) {
        if self.room == CORE_ROOM {
            self.spawn_core_room();
            return;
        }
        if self.room == CACHE_RESET_ROOM {
            self.spawner.room_before = CACHE_RESET_ROOM;
            self.spawner.count_before = 4;
            self.spawner.timer = 4;
            self.enemy_cache = [[0; 21]; 4];
        }

        // Park the enemies of the room just left; bring back the ones cached
        // before that.
        for k in 0..4 {
            let slot = &mut self.entities[k + 1].0[CACHED];
            slot.swap_with_slice(&mut self.enemy_cache[k]);
        }
        let (back_room, back_count) = (self.spawner.room_before, self.spawner.count_before);
        self.spawner.room_before = self.spawner.last_room;
        self.spawner.count_before = self.spawner.count;
        self.spawner.last_room = self.room;
        let timer = self.spawner.timer;
        self.spawner.timer = 0xB4;
        if timer != 0 && self.room == back_room {
            if back_count == 3 {
                self.roll_enemy(4);
            }
            self.spawner.count = 4;
            self.spawn_fixed_specials_only();
            return;
        }

        self.spawner.count = 4;
        self.rng.step();
        self.spawner.seed = self.rng.hi();
        if self.rng.lo() & 0x80 != 0 {
            self.spawner.masks = [0xFF; 4];
            for slot in (1..=4).rev() {
                self.roll_enemy(slot);
            }
        } else {
            self.rng.step();
            self.spawner.params[0] = self.rng.lo() % 15 + 2;
            self.spawner.masks = [0xE0, 0xFF, 0xFF, 0xFF];
            let c = self.rng.hi();
            if c & 1 != 0 {
                self.rng.step();
                self.spawner.params[0] |= (self.rng.lo() % 5 + 2).rotate_right(3);
                self.spawner.masks[0] = 0;
            }
            self.rng.step();
            self.spawner.params[2] = self.rng.lo();
            if c & 2 != 0 {
                self.spawner.masks[2] &= 0x0F;
            }
            if c & 4 != 0 {
                self.spawner.params[2] &= 0xF0;
            }
            if c & 8 != 0 {
                self.spawner.masks[2] &= 0xF0;
            }
            self.spawner.params[3] = self.rng.hi();
            if c & 16 != 0 {
                self.spawner.masks[3] = 0;
            }
            for slot in (1..=4).rev() {
                self.roll_enemy(slot);
            }
            if c & 32 != 0 && c & 64 != 0 {
                self.spawner.masks = [0xFF; 4];
            }
        }
        self.spawn_fixed();
    }

    /// When restoring cached enemies only the special entity is placed
    /// (stationary entities come back with the cache).
    fn spawn_fixed_specials_only(&mut self) {
        if let Some((x, y)) = self.objects.kind12 {
            self.place_special(x, y.wrapping_sub(8));
        }
        let blob = self.entities[0];
        if blob.0[blob::STATE] == 2 {
            self.place_special(blob.x(), blob.y().wrapping_sub(8));
        }
    }
}
