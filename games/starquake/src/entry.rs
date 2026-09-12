//! Entering a room: everything between leaving the last room and the first
//! frame of play in the new one.

use crate::entities::{SLOTS, blob, field};
use crate::game::Game;

/// Why the current room was entered.
pub mod reason {
    /// Walked in, or a new game.
    pub const NORMAL: u8 = 0;
    /// Restarting the room after losing a life.
    pub const RESTART: u8 = 1;
    /// Coming back without re-rolling enemies.
    pub const KEEP_ENEMIES: u8 = 3;
    /// Arriving at a teleporter.
    pub const TELEPORT: u8 = 4;
    pub const ARRIVE_5: u8 = 5;
}

/// Points for the first visit to a room, added to the tens digit.
const FIRST_VISIT_TENS: u8 = 25;

impl Game {
    /// Snaps BLOB to the character grid, draws the panel and clears the
    /// room area.
    pub fn enter_room_prelude(&mut self) {
        let e = &mut self.entities[0].0;
        e[field::Y] = (e[field::Y].wrapping_add(1) & 0xF8).wrapping_sub(1);
        e[field::X] &= 0xF8;
        self.draw_frame();
        self.draw_status();
        self.display.clear_room_area();
    }

    fn marker_position(&self, kind: u8) -> (u8, u8) {
        // The original scans the first 20 marker slots; past the end of the
        // list it reads the (cleared) slot after them.
        let markers = &self.objects.markers;
        markers
            .iter()
            .take(20)
            .find(|m| m.kind == kind)
            .or(markers.get(20))
            .map_or((0, 0), |m| (m.x, m.y))
    }

    /// Enters the current room, up to the first frame of play.
    pub fn enter_room(&mut self) {
        self.enter_room_prelude();
        self.restore_mem[..0xA0].fill(0);
        self.restore_ptr = crate::room::RESTORE_START;
        self.build_room();

        if self.unvisited_rooms.contains(self.room) {
            self.unvisited_rooms.set(self.room, false);
            self.status.pending[4] = FIRST_VISIT_TENS;
            self.add_and_print_score();
        }

        self.rng.a = self.frames as u16;
        self.rng.step();
        for slot in 0..SLOTS {
            self.entities[slot].0[..5].copy_from_slice(&[0x00, 0xDE, 0x40, 0xDF, 0x00]);
        }
        self.platforms.fill(0);
        let b = &mut self.entities[0].0;
        b[blob::FALL] = 0;
        b[blob::PLATFORM_HELD] = 0;
        b[blob::COLOUR] = 7;

        // The core room takes over from here: it needs frames of its own, so
        // the caller with the host runs it (see [`Game::core_room`]).
        if self.room == crate::cores::CORE_ROOM {
            return;
        }

        let arrive = match self.entry_reason {
            reason::TELEPORT => Some(self.marker_position(0x0D)),
            reason::ARRIVE_5 => Some(self.marker_position(0x0F)),
            _ => None,
        };
        let b = &mut self.entities[0].0;
        if let Some((x, y)) = arrive {
            b[field::X] = x;
            b[field::Y] = y;
        }
        if self.entry_reason == reason::RESTART {
            b[field::X] = self.saved_position.0;
            b[field::Y] = self.saved_position.1;
            b[blob::STATE] = self.saved_state;
        }
        self.saved_position = (b[field::X], b[field::Y]);
        self.saved_state = b[blob::STATE];

        if self.entry_reason != reason::KEEP_ENEMIES {
            self.spawn_enemies();
        }
    }
}
