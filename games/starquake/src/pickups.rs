//! Pickups placed when a room is entered: the planet core pieces and other
//! items (45 of them, scattered at the start of a game), and a random bonus.

use crate::game::Game;

/// Number of entries in the item table.
pub const ITEM_COUNT: usize = 45;

/// Rooms without pickups.
const CORE_ROOM: u16 = 199;

/// One entry of the item table, kept in the original's packed form:
/// `[colour << 5 | col, room_high << 7 | row, room_low, graphic]`. A row of
/// 0 means the item has not been put in its room yet.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub struct Item(pub [u8; 4]);

impl Item {
    pub fn room(&self) -> u16 {
        self.0[2] as u16 | ((self.0[1] >> 7) as u16) << 8
    }
    pub fn row(&self) -> u8 {
        self.0[1] & 0x7F
    }
    pub fn col(&self) -> u8 {
        self.0[0] & 0x1F
    }
    pub fn colour(&self) -> u8 {
        self.0[0] >> 5
    }
    pub fn graphic(&self) -> u8 {
        self.0[3]
    }
}

/// The random bonus pickup in the current room.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub struct Bonus {
    pub col: u8,
    pub row: u8,
    /// 0 when there is no bonus.
    pub graphic: u8,
    pub attr: u8,
}

/// A 512-bit set of rooms, most significant bit first.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct RoomSet(pub [u8; 64]);

impl RoomSet {
    pub fn contains(&self, room: u16) -> bool {
        self.0[(room >> 3) as usize] & (0x80 >> (room & 7)) != 0
    }
    pub fn set(&mut self, room: u16, on: bool) {
        let bit = 0x80 >> (room & 7);
        let byte = &mut self.0[(room >> 3) as usize];
        *byte = if on { *byte | bit } else { *byte & !bit };
    }
}

/// Subtracts `n` repeatedly until the result is below `n`. The subtraction
/// happens at least once and wraps, so this is not quite `a % n`.
fn reduce(mut a: u8, n: u8) -> u8 {
    assert!(n != 0, "choosing from an empty set");
    loop {
        a = a.wrapping_sub(n);
        if a < n {
            return a;
        }
    }
}

impl Game {
    /// Places and draws the current room's pickups. Runs after
    /// [`Game::build_room_tiles`].
    pub fn place_pickups(&mut self) {
        self.pickups_in_room = 0;
        if self.room == CORE_ROOM {
            return;
        }

        // Put the first not-yet-placed item that belongs here on a random
        // spawn point, in a random colour.
        let spawns = self.objects.spawn_points.len() as u8;
        let seed_hi = (self.seed >> 8) as u8;
        let chosen = reduce((self.rng.hi() ^ seed_hi) & 0x7F, spawns);
        self.last_spawn_index = chosen;
        let room = self.room;
        if let Some(item) = self.items.iter_mut().find(|i| i.row() == 0 && i.room() == room) {
            let (col, row) = self.objects.spawn_points[chosen as usize];
            let colour = reduce((self.rng.b as u8 ^ self.seed as u8) & 0x3F, 6) + 2;
            item.0[0] = colour.rotate_right(3) | col;
            item.0[1] |= row;
        }

        self.rng.b = self.seed;
        self.bonus = Bonus::default();
        if self.bonus_rooms.contains(room) {
            self.place_bonus();
        }

        for k in 0..crate::pickups::ITEM_COUNT {
            let item = self.items[k];
            if item.room() != room || item.row() < 6 {
                continue;
            }
            let graphic = self.assets.graphic32(item.graphic());
            self.draw_block2x2(&graphic, item.row(), item.col(), item.colour() | 0x40);
            self.add_marker(0x14 + k as u8, item.row(), item.col());
            self.pickups_in_room += 1;
        }

        // An inactive teleporter's pad is blanked.
        if let Some((_, index)) = self.objects.teleport
            && self.teleporters[index].1 & 0x7F == 0 {
                self.blank_teleport_pad();
            }
    }

    /// Prints bright spaces over the room's teleporter pad.
    pub fn blank_teleport_pad(&mut self) {
        let Some(((col, row), _)) = self.objects.teleport else { return };
        let col = (col & 0xFC) | 1;
        for r in row..row + 3 {
            let assets = self.assets.clone();
            self.printer.print(&mut self.display, &assets.font, &assets.udg, &[0x13, 1, 0x16, r, col, b' ']);
        }
    }

    fn place_bonus(&mut self) {
        for _ in 0..20 {
            self.rng.step();
        }
        if self.rng.lo() < 0x55 {
            return;
        }
        let spawns = self.objects.spawn_points.len() as u8;
        let index = loop {
            self.rng.step();
            if spawns == 1 {
                return;
            }
            let i = reduce(self.rng.lo() & 0x7F, spawns);
            if i != self.last_spawn_index {
                break i;
            }
        };
        let (col, row) = self.objects.spawn_points[index as usize];

        // Graphics 0x11–0x19; the last is rarer.
        let kind = loop {
            self.rng.step();
            let k = reduce(self.rng.lo(), 9);
            if k != 8 || self.rng.hi() < 0x7F {
                break k;
            }
        };
        let graphic = kind + 0x11;
        self.rng.step();
        let attr = (reduce(self.rng.hi() & 0x3F, 6) + 2) | 0x40;
        let g = self.assets.graphic32(graphic);
        self.draw_block2x2(&g, row, col, attr);
        self.bonus = Bonus { col, row, graphic, attr };
        self.add_marker(1, row, col);
        self.pickups_in_room += 1;
    }

    /// Draws the room and its pickups.
    pub fn build_room(&mut self) {
        self.build_room_tiles();
        self.place_pickups();
    }
}
