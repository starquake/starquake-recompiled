//! The per-frame sound tick.
//!
//! Two "channels" request effects (`sound[0]` for BLOB's actions,
//! `sound[1]` for enemies and ambience); the tick loads the requested
//! effect's parameters and each frame advances its pitch sweep. The
//! original then toggles the beeper at the resulting pitch for the rest of
//! the frame, which is also what paces the game to 50 Hz.
//!
//! Layout of `sound` (as in the original): request 1, request 2, frames
//! left, pitch, pitch step, wobble, prescaler, prescaler reload.

use crate::game::Game;

/// Effect parameter table in the original (4 bytes per effect).
const EFFECTS: usize = 0xA607;
/// Blocking sound effect parameters (5 bytes per effect).
const BEEPS: usize = 0xD839;

/// T-states per 50 Hz frame.
pub use zx_core::timing::FRAME_T;

/// Speaker changes of a blocking sound effect, as (T-state offset, level)
/// pairs, and its total duration in T-states. Simulates the original
/// routine instruction by instruction, so the pitch and length match.
pub fn beep(ram: &[u8], id: u8) -> (Vec<(u32, bool)>, u32) {
    let p = BEEPS + id.wrapping_mul(5) as usize;
    let ix: [u8; 5] = ram[p..p + 5].try_into().unwrap();
    let mut out = Vec::new();
    // CALL, register saves and table lookup.
    let mut t: u32 = 17 + 147;
    let mut count = ix[4] & 0x1F;
    loop {
        t += 11 + 19 + 19 + 20;
        let h0 = ix[0];
        let mut l = ix[2];
        if ix[4] & 0x40 != 0 {
            t += 7 + 4 + 4 + 20;
            l = if ix[4] & 0x20 != 0 {
                t += 7 + 4 + 12;
                l.wrapping_sub(count)
            } else {
                t += 12 + 4;
                l.wrapping_add(count)
            };
            t += 7 + 4;
            if l == 0 {
                t += 7 + 4;
                l = 1;
            } else {
                t += 12;
            }
        } else {
            t += 12;
        }
        t += 7;
        let mut level = false;
        let mut h = h0;
        loop {
            t += 4;
            let mut b = h;
            loop {
                t += 4 + 11;
                out.push((t, level));
                level = !level;
                t += 7 + 4 + 4 + 4 + 19 + 4 + 20;
                let mut d = (h & b) ^ ix[3];
                if ix[4] & 0x80 != 0 {
                    t += 7 + 4 + 8 + 4 + 7 + 4;
                    d = (d >> 1).wrapping_sub(h) & 0x3F;
                } else {
                    t += 12;
                }
                let loops = if d == 0 { 256 } else { d as u32 };
                t += loops * 45 - 5;
                t += 4 + 4;
                let (next, borrow) = b.overflowing_sub(l);
                if borrow {
                    t += 12;
                    break;
                }
                t += 7 + 4 + 12;
                b = next;
            }
            t += 4 + 19;
            if h == ix[1] {
                t += 12;
                break;
            }
            t += 7;
            if h < ix[1] {
                t += 12 + 4 + 12;
                h = h.wrapping_add(1);
            } else {
                t += 7 + 4 + 12;
                h = h.wrapping_sub(1);
            }
        }
        t += 10 + 4;
        count = count.wrapping_sub(1);
        if count == 0 {
            t += 7;
            break;
        }
        t += 12;
    }
    (out, t + 54)
}

/// T-states between speaker changes for the frame tone returned by
/// [`Game::sound_tick`].
pub fn tone_half_period(half_period: u8) -> u32 {
    let e = if half_period == 0 { 256 } else { half_period as u32 };
    35 * e + 37
}

impl Game {
    fn load_effect(&mut self, id: u8) {
        let at = EFFECTS + id.wrapping_sub(1).rotate_left(2) as usize;
        let p: [u8; 4] = self.assets.ram[at..at + 4].try_into().unwrap();
        let prescale = (p[0] >> 6) + 1;
        self.sound[2..8].copy_from_slice(&[p[0] & 0x3F, p[1], p[2], p[3], prescale, prescale]);
    }

    /// Advances the sound state by a frame. Returns the tone to play for
    /// the rest of the frame, as the original's half-period loop count, or
    /// `None` for silence.
    pub fn sound_tick(&mut self) -> Option<u8> {
        if self.sound[0] != 0 {
            let id = self.sound[0];
            self.sound[0] = 0;
            self.load_effect(id);
        }
        if self.sound[2] == 0 {
            if self.sound[1] == 0 {
                // Now and then an ambient effect is queued for next frame.
                if self.rng.lo() < 4 {
                    self.sound[1] = (self.rng.hi() & 3) + 0x0C;
                }
                return None;
            }
            let id = self.sound[1];
            self.sound[1] = 0;
            self.load_effect(id);
        }
        self.sound[6] = self.sound[6].wrapping_sub(1);
        if self.sound[6] != 0 {
            return None;
        }
        self.sound[6] = self.sound[7];
        self.sound[2] = self.sound[2].wrapping_sub(1);
        self.sound[3] = self.sound[3].wrapping_add(self.sound[4]);
        let half_period = (self.sound[3] ^ self.sound[5]).rotate_right(1) & 0x7F;
        self.sound[5] = self.sound[5].wrapping_add(1);
        Some(half_period)
    }
}
