//! Losing a life.

use crate::entities::field;
use crate::game::Game;
use crate::host::Host;

/// Death reasons at or above this restart BLOB where he entered the room.
const RESTART_FROM_ENTRY: u8 = 0x10;
const OUT_OF_ENERGY: u8 = 2;
/// Explosion direction/animation pairs for the four fragments, in the
/// original.
const FRAGMENTS: usize = 0xC498;
const FRAGMENT_TEMPLATE: usize = 0xC4A0;
const BLANK_GRAPHIC: u16 = 0xDF40;
const EXPLOSION_GRAPHIC: u16 = 0xBEC8;
const BLOB_GRAPHIC: u16 = 0xE074;

impl Game {
    /// Clears the enemy slots.
    fn clear_enemies(&mut self) {
        let count = self.spawner.count as usize;
        if count == 3 {
            self.entities[4].0[field::ANIM_PERIOD] = 2;
            self.entities[4].0[field::ANIM_COUNT] = 2;
        }
        for slot in 1..=count {
            let e = &mut self.entities[slot].0;
            e[field::X] = 0;
            e[field::Y] = 0;
            e[field::GRAPHIC..field::GRAPHIC + 2].copy_from_slice(&BLANK_GRAPHIC.to_le_bytes());
        }
    }

    /// Plays the death sequence for `reason`. Returns `false` if that was
    /// the last life (the game is over); otherwise the caller re-enters the
    /// room.
    /// Plays out a death and returns whether the game goes on.
    ///
    /// # Panics
    ///
    /// If the entity table has been resized, which would be a bug here
    /// rather than anything the player can cause.
    pub fn death_sequence(&mut self, reason: u8, host: &mut dyn Host) -> bool {
        self.entry_reason = if reason >= RESTART_FROM_ENTRY { 1 } else { 0 };
        self.death_kind = reason & 7;

        if self.death_kind == OUT_OF_ENERGY {
            self.clear_enemies();
            self.reset_shot();
            for _ in 0..45 {
                let colour = self.entities[0].0[field::COLOUR] ^ 5;
                self.entities[0].0[field::COLOUR] = colour;
                if self.entities[0].0[0x0A] == 2 {
                    self.entities[4].0[field::COLOUR] = colour;
                }
                self.death_ink ^= 5;
                self.frame_with(host);
                let text = [
                    0x13,
                    1,
                    0x16,
                    1,
                    0x0E,
                    0x15,
                    1,
                    0x10,
                    self.death_ink,
                    b' ',
                    b' ',
                    0x15,
                    0,
                ];
                let assets = self.assets.clone();
                self.printer
                    .print(&mut self.display, &assets.font, &assets.udg, &text);
                self.effects.push(0x0F);
            }
        }

        self.clear_enemies();
        self.reset_shot();
        if self.entities[0].0[0x0A] != 2 {
            let special: [u8; 5] = self.entities[4].0[5..10].try_into().unwrap();
            self.entities[5].0[5..10].copy_from_slice(&special);
        }
        self.spawner.count = 4;
        if self.death_kind != 1 {
            let b = &mut self.entities[0].0;
            b[field::GRAPHIC..field::GRAPHIC + 2].copy_from_slice(&BLANK_GRAPHIC.to_le_bytes());
            b[field::X] &= 0xF8;
        }
        if self.status.lives == 0 {
            self.final_scoring();
        }
        self.effects.push(0x13);

        // Four fragments fly apart, animated by the enemy code.
        let (bx, by) = (self.entities[0].x(), self.entities[0].y());
        let ram = self.assets.clone();
        let ram = &ram.ram;
        for k in 0..4 {
            let e = &mut self.entities[1 + k].0;
            e[field::X] = bx & 0xFE;
            e[field::Y] = by | 1;
            e[field::GRAPHIC..field::GRAPHIC + 2].copy_from_slice(&EXPLOSION_GRAPHIC.to_le_bytes());
            e[field::COLOUR] = 7;
            e[0x0E] = ram[FRAGMENTS + k * 2];
            for (i, &v) in ram[FRAGMENT_TEMPLATE..FRAGMENT_TEMPLATE + 9]
                .iter()
                .enumerate()
            {
                let v = if v == 0xFE {
                    ram[FRAGMENTS + k * 2 + 1]
                } else {
                    v
                };
                if v != 0xFF {
                    e[0x11 + i] = v;
                }
            }
        }
        self.sound[0] = 9;
        for _ in 0..80 {
            self.frame_with(host);
            self.update_enemies();
        }
        self.clear_enemies();
        self.frame_with(host);
        self.pause_frames(host, 50);
        self.reset_shot();

        if self.status.lives == 0 {
            return false;
        }
        self.status.lives -= 1;
        self.status.bars[0] = 0xFF;
        self.status.bars[1] |= 8;
        let b = &mut self.entities[0].0;
        b[field::GRAPHIC..field::GRAPHIC + 2].copy_from_slice(&BLOB_GRAPHIC.to_le_bytes());
        true
    }
}
