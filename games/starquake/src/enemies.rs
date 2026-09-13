//! Enemy behaviour, run once per frame.
//!
//! Each active enemy gets four sub-steps per frame. Every sub-step checks
//! for BLOB's shot and for touching BLOB; the enemy only acts when its
//! animation counter runs out, so its period sets its speed. Acting means
//! advancing its state (appearing → active → exploding), sometimes
//! choosing a new direction according to its behaviour, and moving, bouncing
//! off scenery and the room edges.

use crate::collide::{attr_addr, blocked};
use crate::entities::field::{
    ANIM_COUNT, ANIM_PERIOD, BEHAVIOUR, COLOUR, DIRECTION, GRAPHIC, GRAPHIC_BASE, SPEED_X, SPEED_Y,
    START_X, START_Y, STATE, STATE_COUNT, TURN_COUNT, TURN_PERIOD, X, Y,
};
use crate::game::Game;

/// Direction table in the original (bits: 1 right, 2 left, 4 down, 8 up).
const DIRECTIONS: usize = 0xA2B9;

const APPEAR_GRAPHIC: u16 = 0xB148;
const EXPLODE_GRAPHIC: u16 = 0xBEC8;
const BLANK_GRAPHIC: u16 = 0xDF40;
/// Enemies with graphics at or above this only drain energy on touch.
const HARMLESS_GRAPHICS: u8 = 0xB4;
/// Attribute of cells enemies will not move vertically through.
const ENEMY_BARRIER: u8 = 0x64;

/// Why the update stopped.
enum Step {
    /// Carry on with the next sub-step.
    Next,
    /// Skip the rest of this enemy's sub-steps.
    NextEnemy,
    /// A stationary enemy turned: the original leaves the update early.
    Stop,
    /// BLOB was killed (death reason).
    Killed(u8),
}

fn distance(a: u8, b: u8) -> u8 {
    b.abs_diff(a)
}

impl Game {
    /// Rotated random byte used for per-enemy choices.
    fn enemy_random(&self) -> u8 {
        self.rng.lo().rotate_left(self.enemy_cursor[0] as u32)
    }

    fn direction(&self, i: u8) -> u8 {
        self.assets.ram[DIRECTIONS + i as usize]
    }

    /// Puts BLOB's shot away.
    pub fn reset_shot(&mut self) {
        let s = &mut self.entities[5].0;
        s[X] = 0x00;
        s[Y] = 0x0F;
        s[GRAPHIC..GRAPHIC + 2].copy_from_slice(&BLANK_GRAPHIC.to_le_bytes());
        let b = &mut self.entities[0].0;
        b[0x12] = 0;
        b[0x15] = 0;
        b[0x16] = 0;
    }

    fn enemy_shot(&mut self, slot: usize) {
        let hi = self.entities[slot].0[GRAPHIC + 1];
        self.status.pending[4] = hi.wrapping_sub(0xAE).rotate_left(1);
        self.add_and_print_score();
        self.reset_shot();
        self.request_effect(0x12);
        self.sound[1] = 0x0B;
        let e = &mut self.entities[slot].0;
        e[GRAPHIC..GRAPHIC + 2].copy_from_slice(&EXPLODE_GRAPHIC.to_le_bytes());
        e[COLOUR] = 7;
        e[STATE] = 2;
        e[STATE_COUNT] = 0;
    }

    /// Returns a death reason if the enemy in `slot` kills BLOB.
    fn enemy_touches_blob(&mut self, slot: usize) -> Option<u8> {
        self.work.proximity_checks += 1;
        let e = self.entities[slot];
        let b = self.entities[0];
        if distance(e.x(), b.x()) >= 14 || distance(e.y(), b.y()) >= 11 {
            return None;
        }
        if e.0[GRAPHIC + 1] >= HARMLESS_GRAPHICS {
            let drain = &mut self.entities[0].0[0x18];
            *drain = drain.wrapping_add(10);
            return None;
        }
        let blob = &mut self.entities[0].0;
        blob[GRAPHIC..GRAPHIC + 2].copy_from_slice(&e.0[GRAPHIC..GRAPHIC + 2]);
        blob[COLOUR] = e.0[COLOUR];
        Some(if e.0[GRAPHIC] == 0xC8 { 0x11 } else { 0x01 })
    }

    fn enemy_respawn(&mut self, slot: usize) {
        let e = &mut self.entities[slot].0;
        e[X] = 0;
        e[Y] = 0;
        e[GRAPHIC..GRAPHIC + 2].copy_from_slice(&BLANK_GRAPHIC.to_le_bytes());
        if self.rng.lo() & 0xE0 < self.spawner.seed {
            return;
        }
        self.entities[slot].0[Y] = 0x0F;
        self.roll_enemy(slot);
    }

    fn chase_blob(&mut self, slot: usize) {
        let b = self.entities[0];
        let e = &mut self.entities[slot].0;
        let horizontal = if e[X] < b.x() { 1 } else { 2 };
        let vertical = if e[Y] < b.y() { 8 } else { 4 };
        e[DIRECTION] = vertical | horizontal;
    }

    fn random_direction_and_speed(&mut self, slot: usize) {
        let dir = self.direction(self.enemy_random() & 7);
        self.entities[slot].0[DIRECTION] = dir;
        if (dir & 0x0F).count_ones() == 1 {
            return;
        }
        let r = self.enemy_random();
        if r & 0x80 != 0 {
            return;
        }
        let bit = (r >> 6) & 1;
        let e = &mut self.entities[slot].0;
        e[SPEED_X] = bit + 1;
        e[SPEED_Y] = (bit ^ 1) + 1;
    }

    /// Picks a new direction. Returns `false` when the move should skip the
    /// random turn-timer step, and `None` when the update must stop.
    fn enemy_turn(&mut self, slot: usize) -> Option<bool> {
        let behaviour = self.entities[slot].0[BEHAVIOUR];
        let set_speed = |g: &mut Game| {
            g.entities[slot].0[SPEED_X] = 2;
            g.entities[slot].0[SPEED_Y] = 2;
        };
        match behaviour {
            0 => {
                set_speed(self);
                let e = &mut self.entities[slot].0;
                let mut c = e[DIRECTION];
                if c & 0x0B == 0 {
                    c |= 2;
                }
                if c & 0x0C == 0 {
                    c |= 8;
                }
                e[DIRECTION] = c;
                return Some(false);
            }
            1 => {
                set_speed(self);
                let d = self.direction((self.enemy_random() & 3) << 1);
                self.entities[slot].0[DIRECTION] = d;
            }
            2 => {
                set_speed(self);
                let d = self.direction(self.enemy_random() & 7);
                self.entities[slot].0[DIRECTION] = d;
            }
            3 => {
                set_speed(self);
                self.random_direction_and_speed(slot);
            }
            4 => self.chase_blob(slot),
            5 => {
                self.entities[slot].0[DIRECTION] = 0;
                let r = self.enemy_random();
                if r & 1 != 0 {
                    return Some(false);
                }
                if r >> 1 < 0x46 {
                    self.chase_blob(slot);
                } else {
                    set_speed(self);
                    self.random_direction_and_speed(slot);
                }
            }
            6 => {
                let e = &mut self.entities[slot].0;
                let d = e[DIRECTION] & 3;
                e[DIRECTION] = if d == 0 { 1 } else { d };
                set_speed(self);
                return None;
            }
            _ => return Some(false),
        }
        Some(true)
    }

    fn enemy_move(&mut self, slot: usize) {
        let x = self.entities[slot].x();
        let dir = self.entities[slot].0[DIRECTION];
        if x < 3 {
            self.entities[slot].0[DIRECTION] = (dir & 0xFC) | 1;
        } else if x >= 0xEE {
            self.entities[slot].0[DIRECTION] = (dir & 0xFC) | 2;
        } else {
            let f = self.collide(slot, false) & (blocked::LEFT | blocked::RIGHT);
            if f != 0 {
                let e = &mut self.entities[slot].0;
                e[DIRECTION] = (e[DIRECTION] & 0xFC) | (f ^ 3);
            }
        }
        let e = &mut self.entities[slot].0;
        let mut x = e[X];
        if e[DIRECTION] & 1 != 0 {
            x = x.wrapping_add(e[SPEED_X]);
        }
        if e[DIRECTION] & 2 != 0 {
            x = x.wrapping_sub(e[SPEED_X]);
        }
        e[X] = x;

        let y = e[Y];
        if (x | y.wrapping_add(1)) & 7 == 0 {
            let at = attr_addr(x, y.wrapping_add(1));
            if [0, 1, 33, 32]
                .iter()
                .any(|&o| self.screen_byte(at + o) == ENEMY_BARRIER)
            {
                return;
            }
        }

        let dir = self.entities[slot].0[DIRECTION];
        if y < 0x12 {
            self.entities[slot].0[DIRECTION] = (dir & 0xF3) | 8;
        } else if y >= 0x8D {
            self.entities[slot].0[DIRECTION] = (dir & 0xF3) | 4;
        } else if self.entities[slot].0[BEHAVIOUR] != 6 {
            let f = self.collide(slot, true) & (blocked::ABOVE | blocked::BELOW);
            if f != 0 {
                let e = &mut self.entities[slot].0;
                e[DIRECTION] = (e[DIRECTION] & 0xF3) | (f ^ 0x0C);
            }
        }
        let e = &mut self.entities[slot].0;
        let mut y = e[Y];
        if e[DIRECTION] & 4 != 0 {
            y = y.wrapping_sub(e[SPEED_Y]);
        }
        if e[DIRECTION] & 8 != 0 {
            y = y.wrapping_add(e[SPEED_Y]);
        }
        e[Y] = y;
    }

    fn enemy_substep(&mut self, slot: usize) -> Step {
        let e = self.entities[slot];
        if e.0[STATE] != 2 && (e.0[STATE] | e.0[STATE_COUNT]) != 0 {
            let shot = self.entities[5];
            if distance(e.x(), shot.x()) < 14 && distance(e.y(), shot.y()) < 14 {
                self.enemy_shot(slot);
            }
        }
        if self.entities[slot].0[STATE] == 1
            && let Some(reason) = self.enemy_touches_blob(slot)
        {
            return Step::Killed(reason);
        }

        let e = &mut self.entities[slot].0;
        e[ANIM_COUNT] = e[ANIM_COUNT].wrapping_sub(1);
        if e[ANIM_COUNT] != 0 {
            return Step::Next;
        }
        e[ANIM_COUNT] = e[ANIM_PERIOD];
        match e[STATE] {
            0 => {
                let old = e[STATE_COUNT];
                e[STATE_COUNT] = old.wrapping_add(1);
                if old == 0 {
                    e[GRAPHIC..GRAPHIC + 2].copy_from_slice(&APPEAR_GRAPHIC.to_le_bytes());
                    e[X] = e[START_X];
                    e[Y] = e[START_Y];
                    self.sound[1] = (self.rng.lo() & 3) + 1;
                } else if old == 0x10 {
                    e[STATE] = 1;
                    e[STATE_COUNT] = 0;
                    let (b0, b1) = (e[GRAPHIC_BASE], e[GRAPHIC_BASE + 1]);
                    e[GRAPHIC] = b0;
                    e[GRAPHIC + 1] = b1;
                }
            }
            2 => {
                e[STATE_COUNT] = e[STATE_COUNT].wrapping_add(1);
                if e[STATE_COUNT] == 8 {
                    self.enemy_respawn(slot);
                    return Step::NextEnemy;
                }
            }
            _ => {}
        }

        let e = &mut self.entities[slot].0;
        e[TURN_COUNT] = e[TURN_COUNT].wrapping_sub(1);
        if e[TURN_COUNT] == 0 {
            e[TURN_COUNT] = e[TURN_PERIOD];
            match self.enemy_turn(slot) {
                None => return Step::Stop,
                Some(true) => {
                    if self.entities[slot].0[TURN_PERIOD] == 0x64 {
                        self.entities[slot].0[TURN_COUNT] = ((self.rng.hi() & 3) + 1) << 1;
                    }
                }
                Some(false) => {}
            }
        }
        self.enemy_move(slot);
        Step::Next
    }

    /// Runs one frame of enemy behaviour. Returns a death reason if an enemy
    /// killed BLOB.
    pub fn update_enemies(&mut self) -> Option<u8> {
        if self.spawner.timer != 0 {
            self.spawner.timer -= 1;
        }
        self.rng.step();
        self.enemy_cursor[0] = self.spawner.count;
        loop {
            let slot = self.enemy_cursor[0] as usize;
            if self.entities[slot].y() != 0 {
                self.enemy_cursor[1] = 4;
                loop {
                    match self.enemy_substep(slot) {
                        Step::Next => {}
                        Step::NextEnemy => break,
                        Step::Stop => return None,
                        Step::Killed(reason) => return Some(reason),
                    }
                    self.enemy_cursor[1] -= 1;
                    if self.enemy_cursor[1] == 0 {
                        break;
                    }
                }
            }
            self.enemy_cursor[0] -= 1;
            if self.enemy_cursor[0] == 0 {
                return None;
            }
        }
    }
}
