//! BLOB: controls, movement, shooting, and what happens when he touches
//! things. Runs once per frame after the display work.

use crate::collide::blocked;
use crate::controls::Input;
use crate::entities::field::{COLOUR, GRAPHIC, X, Y};
use crate::game::Game;

/// BLOB's fields in slot 0.
pub mod b {
    pub const STATE: usize = 0x0A;
    /// Direction bits read this frame, with opposite directions cancelled
    /// and diagonals flagged in bits 4–7.
    pub const INPUT: usize = 0x0B;
    pub const LAST_INPUT: usize = 0x0C;
    pub const WALK_FRAME: usize = 0x0D;
    /// 0 facing right … 4 facing left.
    pub const FACING: usize = 0x0E;
    pub const FIRE: usize = 0x0F;
    pub const STEP: usize = 0x10;
    pub const FALL: usize = 0x11;
    pub const SHOT_DIR: usize = 0x12;
    pub const SHOT_FACING: usize = 0x13;
    pub const PLATFORM_HELD: usize = 0x14;
    pub const HOVER_SHOT: usize = 0x15;
    pub const HOVER_BOUNCES: usize = 0x16;
    pub const SHOT_FRAME: usize = 0x17;
    pub const DRAIN: usize = 0x18;
    pub const PICKUP: usize = 0x19;
    pub const PICKUP_HELD: usize = 0x1A;
}

/// States (`b::STATE`).
const WALKING: u8 = 0;
const LIFTED: u8 = 1;
const HOVERING: u8 = 2;

/// Tables in the original.
const FALL_SPEEDS: usize = 0xC751;
const HOVER_GRAPHICS: usize = 0xCA0B;
const SHOT_GRAPHICS: usize = 0xCB2B;
const BONUS_EFFECTS: usize = 0xCCBC;

const WALK_RIGHT: u16 = 0xE074;
const WALK_LEFT: u16 = 0xE374;
const TURNING: u16 = 0xE674;
const LIFT_GRAPHIC: u16 = 0xBF88;
const SHOT_RIGHT: u16 = 0xE8B4;
const SHOT_LEFT: u16 = 0xE974;
const HOVER_GRAPHIC: u16 = 0xAFC8;
/// Attribute of lift cells and other special floor.
pub(crate) const SPECIAL_CELL: u8 = 0x64;
const CHEOPS: u8 = 0x19;

/// Screens that take over the game until the player is done with them.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Modal {
    SecurityDoor,
    Cheops,
    TeleportBooth,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Outcome {
    Continue,
    /// Enter the (already changed) current room, for this reason.
    NewRoom(u8),
    /// BLOB died (death reason).
    Died(u8),
    Modal(Modal),
    /// A, S, D, F and G held together: abandon the game.
    Quit,
    /// P pressed.
    Pause,
}

fn distance(a: u8, b: u8) -> u8 {
    b.abs_diff(a)
}

impl Game {
    fn blob(&self, f: usize) -> u8 {
        self.entities[0].0[f]
    }

    fn set_blob(&mut self, f: usize, v: u8) {
        self.entities[0].0[f] = v;
    }

    fn set_graphic(&mut self, slot: usize, g: u16) {
        self.entities[slot].0[GRAPHIC..GRAPHIC + 2].copy_from_slice(&g.to_le_bytes());
    }

    fn ram_byte(&self, addr: usize) -> u8 {
        self.assets.ram[addr]
    }

    fn footstep(&mut self) {
        self.footstep_sound ^= 1;
        self.request_effect(self.footstep_sound);
    }

    /// Advances walking animation after a step in `right` or left direction.
    fn walk_step(&mut self, right: bool) {
        let step = self.blob(b::STEP).wrapping_add(1);
        self.set_blob(b::STEP, step);
        if step < 3 {
            return;
        }
        self.set_blob(b::STEP, 0);
        let facing = self.blob(b::FACING);
        let (end, next) = if right {
            (0, facing.wrapping_sub(1))
        } else {
            (4, facing + 1)
        };
        if facing != end {
            self.set_blob(b::FACING, next);
            self.set_blob(b::WALK_FRAME, 0);
            if next != end {
                self.set_graphic(0, TURNING.wrapping_add((next - 1) as u16 * 0xC0));
                return;
            }
        }
        self.footstep();
        let frame = (self.blob(b::WALK_FRAME) + 1) % 4;
        self.set_blob(b::WALK_FRAME, frame);
        let base = if right { WALK_RIGHT } else { WALK_LEFT };
        self.set_graphic(0, base.wrapping_add(frame as u16 * 0xC0));
    }

    /// Being carried up a lift shaft until BLOB can step out sideways.
    fn lifted(&mut self) {
        let input = self.blob(b::INPUT) & !8;
        self.set_blob(b::INPUT, input);
        if self.collide(0, true) & blocked::ABOVE == 0 {
            self.set_blob(Y, self.blob(Y).wrapping_add(2));
        }
        self.set_graphic(0, LIFT_GRAPHIC);
        if self.collide(0, false) & 3 != 3 {
            self.set_blob(b::STATE, WALKING);
            self.set_blob(b::STEP, 2);
        }
    }

    fn build_platform(&mut self) {
        // A pad's D-pad down only flies the hover platform down (#112).
        if self.blob(b::PLATFORM_HELD) != 0 || self.status.bars[1] == 0 || self.pad.down_moves_only
        {
            return;
        }
        self.set_blob(b::PLATFORM_HELD, 1);
        let c = self.collide(0, true);
        let y = self.blob(Y);
        let mut at = u16::from_le_bytes([self.collision[1], self.collision[2]]).wrapping_add(64);
        if y.wrapping_add(1) & 7 != 0 {
            at = at.wrapping_add(32);
        }
        let bright = |g: &Game, a: u16| g.screen_byte(a) & 0x40 != 0;
        let supported = if y < 0x17 {
            self.set_blob(Y, 0x0F);
            false
        } else if !bright(self, at) {
            false
        } else if !bright(self, at + 1) {
            at += 1;
            false
        } else {
            at += 2;
            bright(self, at)
        };
        if !supported {
            if c & blocked::ABOVE != 0 {
                return;
            }
            let y = self.blob(Y);
            self.set_blob(Y, (y.wrapping_add(1) & 0xF8).wrapping_add(7));
        }
        if self.screen_byte(at) & 0x7F == SPECIAL_CELL {
            return;
        }
        let Some(k) = (0..12).find(|k| self.platforms[k * 4 + 1] == 0) else {
            return;
        };
        let col = (self.blob(X).wrapping_add(4) & 0xF8) >> 3;
        let row = (0xD6u8.wrapping_sub(self.blob(Y)) & 0xF8) >> 3;
        self.platforms[k * 4] = col;
        self.platforms[k * 4 + 1] = row;
        self.platforms[k * 4 + 3] = (self.rng.lo() & 3) + 5;
        for frame in 0..4 {
            self.draw_strip(frame, row, col, 0);
        }
        self.reduce_bar(1, 2);
        self.sound[0] = 8;
    }

    /// Walking (and falling, and building platforms).
    fn walking(&mut self) {
        let side = self.collide(0, false);
        let input = self.blob(b::INPUT);
        if input & 1 != 0 {
            if side & blocked::RIGHT == 0 {
                self.set_blob(X, self.blob(X).wrapping_add(2));
            }
            self.walk_step(true);
        } else if input & 2 != 0 {
            if side & blocked::LEFT == 0 {
                self.set_blob(X, self.blob(X).wrapping_sub(2));
            }
            self.walk_step(false);
        }

        // Standing on the hover platform's pad: no jumping onto it.
        let pad = self.objects.kind12.unwrap_or((0, 0));
        if pad == (self.blob(X), self.blob(Y)) {
            self.set_blob(b::INPUT, self.blob(b::INPUT) & !8);
            return;
        }

        let v = self.collide(0, true);
        let (x, y) = (self.blob(X), self.blob(Y));
        if x.wrapping_sub(8) & 0x1F == 0 && y % 3 == 0 {
            let under = u16::from_le_bytes([self.collision[1], self.collision[2]]).wrapping_add(33);
            if self.screen_byte(under) == SPECIAL_CELL {
                self.set_blob(b::STATE, LIFTED);
                self.lifted();
                return;
            }
        }

        if v & blocked::BELOW == 0 {
            if self.blob(b::FALL) == 0 {
                self.sound[0] = 6;
            }
            let fall = self.blob(b::FALL);
            let fall = if fall == 0x10 { fall } else { fall + 1 };
            self.set_blob(b::FALL, fall);
            let drop = self.ram_byte(FALL_SPEEDS + fall as usize - 1);
            self.set_blob(Y, self.blob(Y).wrapping_sub(drop));
        } else if self.blob(b::FALL) != 0 {
            self.sound[0] = 7;
            self.set_blob(b::FALL, 0);
        }

        if self.blob(b::INPUT) == 4 {
            self.build_platform();
        } else {
            self.set_blob(b::PLATFORM_HELD, 0);
        }
    }

    /// Fires and moves the straight shot.
    fn straight_shot(&mut self) {
        if self.blob(b::SHOT_DIR) == 0 {
            if self.blob(b::FIRE) == 0 || self.status.bars[2] == 0 || self.sound_busy() {
                return;
            }
            self.sound[0] = 5;
            self.reduce_bar(2, 1);
            let dir = self.blob(b::SHOT_FACING);
            self.set_blob(b::SHOT_DIR, dir);
            self.set_graphic(5, if dir & 1 != 0 { SHOT_RIGHT } else { SHOT_LEFT });
            let (x, y) = (self.blob(X), self.blob(Y));
            self.entities[5].0[X] = x;
            self.entities[5].0[Y] = y;
        }
        let dir = self.blob(b::SHOT_DIR);
        for _ in 0..3 {
            let hit = if self.entities[5].x() & 7 == 0 {
                self.collide(5, false)
            } else {
                0
            };
            if hit & dir != 0 {
                self.reset_shot();
                return;
            }
            let x = self.entities[5].x();
            let x = if self.blob(b::SHOT_DIR) == 1 {
                x.wrapping_add(2)
            } else {
                x.wrapping_sub(2)
            };
            self.entities[5].0[X] = x;
            if x >= 0xF2 {
                self.reset_shot();
                return;
            }
        }
    }

    /// Whether a long sound effect is playing that firing must not cut off.
    fn sound_busy(&self) -> bool {
        self.sound[2] != 0 && self.sound[4] == 0xF7
    }

    /// Riding the hover platform: free movement in all directions.
    fn hovering(&mut self) {
        // A pad's button for up picks up and its button for down builds:
        // neither flies (#112).
        let mut input = self.blob(b::INPUT);
        if self.pad.up_picks_only {
            input &= !8;
        }
        if self.pad.down_builds_only {
            input &= !4;
        }
        let free = |g: &mut Game, vertical: bool| {
            let a = g.collide(0, vertical);
            g.set_blob(Y, g.blob(Y).wrapping_sub(8));
            let c = g.collide(0, vertical);
            g.set_blob(Y, g.blob(Y).wrapping_add(8));
            (input & (a | c)) ^ input
        };
        let e = free(self, true);
        let mut y = self.blob(Y);
        if e & 8 != 0 {
            y = y.wrapping_add(2);
        }
        if e & 4 != 0 {
            y = y.wrapping_sub(2);
        }
        self.set_blob(Y, y);

        let (x, y) = (self.blob(X), self.blob(Y));
        self.entities[4].0[X] = x;
        self.entities[4].0[Y] = y.wrapping_sub(8);
        self.set_graphic(4, HOVER_GRAPHIC);
        self.entities[4].0[COLOUR] = 7;

        let e = free(self, false);
        let mut x = self.blob(X);
        if e & 1 != 0 {
            x = x.wrapping_add(2);
        }
        if e & 2 != 0 {
            x = x.wrapping_sub(2);
        }
        self.set_blob(X, x);

        let step = self.blob(b::STEP) + 1;
        self.set_blob(b::STEP, step);
        if step >= 3 {
            self.set_blob(b::STEP, 0);
            let mut facing = self.blob(b::FACING);
            if input & 1 != 0 {
                facing = facing.wrapping_sub(1);
                if facing >= 5 {
                    facing = 0;
                }
            }
            if input & 2 != 0 {
                facing += 1;
                if facing >= 5 {
                    facing = 4;
                }
            }
            self.set_blob(b::FACING, facing);
            let g = u16::from_le_bytes([
                self.ram_byte(HOVER_GRAPHICS + facing as usize * 2),
                self.ram_byte(HOVER_GRAPHICS + facing as usize * 2 + 1),
            ]);
            self.set_graphic(0, g);
        }
    }

    /// The hover platform's shot, which travels in eight directions and
    /// bounces off scenery once.
    fn bouncing_shot(&mut self) {
        if self.blob(b::HOVER_SHOT) == 0 {
            if self.blob(b::FIRE) == 0 || self.status.bars[2] == 0 || self.sound_busy() {
                return;
            }
            self.sound[0] = 5;
            self.reduce_bar(2, 1);
            let (x, y) = (self.blob(X), self.blob(Y));
            self.entities[5].0[X] = x & 0xF8;
            self.entities[5].0[Y] = (y.wrapping_add(1) & 0xF8).wrapping_sub(1);
            self.set_blob(b::HOVER_SHOT, self.blob(b::LAST_INPUT));
        }

        let c = self.collide(5, false);
        let mut bounces = 0u8;
        let mut d = self.blob(b::HOVER_SHOT);
        if d & 3 & c != 0 {
            bounces = 1;
            d ^= 3;
            if d & 0x0C == 0 {
                d |= 8;
                if self.rng.lo() & 1 != 0 {
                    d ^= 0x0C;
                }
            }
            self.set_blob(b::HOVER_SHOT, d);
        } else {
            let s = &mut self.entities[5].0;
            if d & 1 != 0 {
                s[X] = s[X].wrapping_add(8);
            }
            if d & 2 != 0 {
                s[X] = s[X].wrapping_sub(8);
            }
        }

        let c = self.collide(5, true);
        let mut d = self.blob(b::HOVER_SHOT);
        if d & 0x0C & c != 0 {
            bounces += 1;
            d ^= 0x0C;
            if d & 3 == 0 {
                d |= 1;
                if self.rng.hi() & 1 != 0 {
                    d ^= 3;
                }
            }
            if bounces == 2 {
                bounces = 1;
                d &= if self.rng.lo() & 0x20 != 0 { 3 } else { 0x0C };
            }
            self.set_blob(b::HOVER_SHOT, d);
        } else {
            let s = &mut self.entities[5].0;
            if d & 4 != 0 {
                s[Y] = s[Y].wrapping_sub(8);
            }
            if d & 8 != 0 {
                s[Y] = s[Y].wrapping_add(8);
            }
        }

        let total = self.blob(b::HOVER_BOUNCES) + bounces;
        if total >= 2 {
            self.reset_shot();
            return;
        }
        self.set_blob(b::HOVER_BOUNCES, total);
        let (sx, sy) = (self.entities[5].x(), self.entities[5].y());
        if sx >= 0xF2 || !(0x0F..0x91).contains(&sy) {
            self.reset_shot();
            return;
        }
        let frame = (self.blob(b::SHOT_FRAME) + 1) % 4;
        self.set_blob(b::SHOT_FRAME, frame);
        let g = u16::from_le_bytes([
            self.ram_byte(SHOT_GRAPHICS + frame as usize * 2),
            self.ram_byte(SHOT_GRAPHICS + frame as usize * 2 + 1),
        ]);
        self.set_graphic(5, g);
    }

    fn change_room(&mut self, delta: i16) -> Outcome {
        self.room = self.room.wrapping_add(delta as u16);
        let y = self.blob(Y);
        self.set_blob(Y, (y.wrapping_add(1) & 0xF8).wrapping_sub(1));
        self.reset_shot();
        Outcome::NewRoom(0)
    }

    /// Leaving through an edge of the room.
    fn room_exit(&mut self) -> Option<Outcome> {
        let input = self.blob(b::INPUT);
        let x = self.blob(X);
        if x.wrapping_sub(0xF0) < 4 && input & 1 != 0 {
            self.set_blob(X, 0);
            return Some(self.change_room(1));
        }
        if x.wrapping_add(2) < 4 && input & 2 != 0 {
            self.set_blob(X, 0xF0);
            return Some(self.change_room(-1));
        }
        let y = self.blob(Y);
        if y < 0x0E {
            self.set_blob(Y, 0x8F);
            return Some(self.change_room(16));
        }
        if y >= 0x90 {
            self.set_blob(Y, 0x0F);
            return Some(self.change_room(-16));
        }
        None
    }

    /// Bonus pickups: 0x11–0x16 refill a bar, 0x17 whatever is lowest,
    /// 0x18 a life.
    fn apply_bonus(&mut self, kind: u8) {
        let kind = if kind == 0x17 {
            self.lowest_refill()
        } else {
            kind
        };
        let i = BONUS_EFFECTS + (kind.wrapping_sub(0x11) as usize) * 2;
        let (field, amount) = (self.ram_byte(i), self.ram_byte(i + 1));
        self.request_effect(field);
        let target = match field {
            0 => &mut self.status.lives,
            n => &mut self.status.bars[n as usize - 1],
        };
        *target = target.wrapping_add(amount);
        self.draw_status();
    }

    fn lowest_refill(&self) -> u8 {
        if self.status.lives == 0 {
            return 0x18;
        }
        let mut low = 0xFF;
        let mut which = 0;
        for (i, &bar) in self.status.bars.iter().enumerate() {
            if low >= bar {
                which = i as u8 * 2;
                low = bar;
            }
        }
        which + 0x12
    }

    /// Whether the 2 × 2 cells at (row, col) are bright and not special.
    fn cells_free(&self, row: u8, col: u8) -> bool {
        let at = 0x5800
            + ((row.rotate_right(3) & 3) as u16) * 256
            + ((row.rotate_right(3) & 0xE0) | col) as u16;
        [0, 1, 33, 32].iter().all(|&o| {
            let a = self.screen_byte(at + o);
            a & 0x40 != 0 && a != SPECIAL_CELL
        })
    }

    fn find_item_at_row(&self, row: u8) -> Option<usize> {
        self.items.iter().position(|i| i.0[1] & 0x7F == row)
    }

    /// Picking up rotates the inventory; an item pushed out is dropped next
    /// to BLOB.
    fn rotate_inventory(&mut self) {
        let s = &mut self.status;
        if s.outgoing.1 != 0 && self.pickups_in_room >= 4 {
            return;
        }
        self.request_effect(0x0C);
        let s = &mut self.status;
        s.outgoing = s.inventory[3];
        s.inventory = [s.incoming, s.inventory[0], s.inventory[1], s.inventory[2]];
        s.incoming = (0, 0);
        for row in (1..=5).rev() {
            if let Some(k) = self.find_item_at_row(row) {
                self.items[k].0[1] = if row + 1 == 6 { 0x32 } else { row + 1 };
            }
        }
        self.draw_status();

        let (graphic, attr) = self.status.outgoing;
        if attr == 0 {
            return;
        }
        self.pickups_in_room += 1;
        let col = (self.blob(X) >> 3) & 0x1F;
        let row = 0xBFu8.wrapping_sub(self.blob(Y)).rotate_right(3) & 0x1F;
        let mut c = col;
        let mut placed = false;
        if c >= 1 {
            c -= 1;
            if self.cells_free(row, c) {
                placed = true;
            } else {
                c += 1;
            }
        }
        if !placed && c < 0x1D {
            c += 2;
            if !self.cells_free(row, c) {
                c -= 2;
            }
        }
        let k = self
            .find_item_at_row(0x32)
            .expect("the dropped item is in the table");
        let item = &mut self.items[k].0;
        item[0] = (item[0] & 0xE0) | c;
        item[1] = ((self.room >> 8) as u8).rotate_right(1) | row;
        item[2] = self.room as u8;
        item[3] = graphic;
        let g = self.assets.graphic32(graphic);
        self.draw_block2x2(&g, row, c, attr.wrapping_add(0x40));
        self.add_marker(0x14 + k as u8, row, c);
    }

    /// Checks BLOB against every marker in the room.
    fn touch(&mut self) -> Outcome {
        let drain = self.blob(b::DRAIN).wrapping_add(1);
        self.set_blob(b::DRAIN, drain);
        if drain >= 0x78 {
            self.set_blob(b::DRAIN, 0);
            self.reduce_bar(0, 4);
        }
        // A pad's D-pad up boards and flies, and never picks up (#112).
        if self.blob(b::INPUT) == 8 && !self.pad.up_moves_only {
            if self.blob(b::PICKUP_HELD) == 0 {
                self.set_blob(b::PICKUP, 1);
                self.set_blob(b::PICKUP_HELD, 1);
            } else {
                self.set_blob(b::PICKUP, 0);
            }
        } else {
            self.set_blob(b::PICKUP, 0);
            self.set_blob(b::PICKUP_HELD, 0);
        }

        let mut i = 0;
        for _ in 0..22 {
            let Some(&m) = self.objects.markers.get(i) else {
                break;
            };
            let (bx, by) = (self.blob(X), self.blob(Y));
            let exact = m.kind == 0 || ((0x0C..0x14).contains(&m.kind) && m.kind != 0x0E);
            let hit = if exact {
                bx == m.x && by == m.y
            } else {
                distance(m.y, by) < 15 && distance(m.x, bx) < 15
            };
            let mut removed = false;
            if hit {
                match self.touch_marker(i) {
                    Ok(r) => removed = r,
                    Err(outcome) => return outcome,
                }
            }
            if !removed {
                i += 1;
            }
            if self.objects.markers.get(i).is_none_or(|m| m.y == 0) {
                break;
            }
        }

        if self.blob(b::PICKUP) != 0 {
            self.rotate_inventory();
        }
        self.display.border = 0;
        Outcome::Continue
    }

    /// Handles touching marker `i`. Returns whether it was removed from the
    /// list, or an outcome that ends the frame.
    fn touch_marker(&mut self, i: usize) -> Result<bool, Outcome> {
        let m = self.objects.markers[i];
        let input = self.blob(b::INPUT);
        match m.kind {
            0 if input & 3 != 0 => Err(Outcome::Modal(Modal::SecurityDoor)),
            1 => {
                let bonus = self.bonus;
                if bonus.graphic == CHEOPS {
                    return if input & 8 != 0 {
                        Err(Outcome::Modal(Modal::Cheops))
                    } else {
                        Ok(false)
                    };
                }
                let g = self.assets.graphic32(bonus.graphic);
                self.draw_block2x2(&g, bonus.row, bonus.col, 0x47);
                self.remove_restore_block(bonus.row, bonus.col, bonus.attr | 0x40);
                self.objects.markers[i].kind = 5;
                self.pickups_in_room = self.pickups_in_room.wrapping_sub(1);
                self.bonus_rooms.set(self.room, false);
                self.apply_bonus(bonus.graphic);
                Ok(false)
            }
            6 => Err(Outcome::Died(0x10)),
            0x0B => {
                if self.status.inventory.iter().any(|&(g, _)| g == 0x10)
                    && let Some((_, index)) = self.objects.teleport
                {
                    let entry = &mut self.teleporters[index].1;
                    if *entry & 0x7F != 0 {
                        *entry &= 0x80;
                        self.blank_teleport_pad();
                        self.request_effect(8);
                    }
                }
                Ok(false)
            }
            0x0C => {
                self.reset_shot();
                // A pad's button for up picks up and never boards (#112).
                let state = if self.blob(b::LAST_INPUT) & 8 != 0 && !self.pad.up_picks_only {
                    HOVERING
                } else {
                    WALKING
                };
                self.set_blob(b::STATE, state);
                Ok(false)
            }
            0x0D if input & 3 != 0 => Err(Outcome::Modal(Modal::TeleportBooth)),
            0x0E => {
                if self.blob(b::FALL) != 0x10 || self.blob(Y) != m.y {
                    return Ok(false);
                }
                self.objects.markers[i].kind = 5;
                let mut col = m.x.rotate_right(3).wrapping_sub(1) & 0x1F;
                let row = 0xBFu8.wrapping_sub(m.y).rotate_right(3).wrapping_add(2) & 0x1F;
                let assets = self.assets.clone();
                self.work.characters += self.printer.print(
                    &mut self.display,
                    &assets.font,
                    &assets.udg,
                    &[0x16, row, col, 0x13, 1, 0x10, 7, b' ', b' ', b' ', b' '],
                );
                self.request_effect(0x10);
                for _ in 0..2 {
                    let Some(k) = (0..12).find(|k| self.platforms[k * 4 + 1] == 0) else {
                        return Ok(false);
                    };
                    self.platforms[k * 4] = col | 0x40;
                    self.platforms[k * 4 + 1] = row;
                    self.platforms[k * 4 + 3] = 2;
                    self.draw_strip(3, row, col, 0);
                    self.draw_strip(2, row, col, 0);
                    col = col.wrapping_add(2);
                }
                Ok(false)
            }
            0x0F if input & 3 != 0 => {
                self.room = if input & 1 != 0 {
                    self.room.wrapping_add(1)
                } else {
                    self.room.wrapping_sub(1)
                };
                self.request_effect(4);
                Err(Outcome::NewRoom(5))
            }
            k if k >= 0x14 => {
                if self.blob(b::PICKUP) != 1 {
                    return Ok(false);
                }
                self.set_blob(b::PICKUP, 2);
                self.pickups_in_room = self.pickups_in_room.wrapping_sub(1);
                let item = (k - 0x14) as usize;
                let [b0, b1, _, graphic] = self.items[item].0;
                let (col, colour) = (b0 & 0x1F, b0 >> 5);
                self.items[item].0[1] = 1;
                self.status.incoming = (graphic, colour);
                let g = self.assets.graphic32(graphic);
                self.draw_block2x2(&g, b1, col, 0x47);
                self.remove_restore_block(b1, col, colour | 0x40);
                self.objects.markers.remove(i);
                Ok(true)
            }
            _ => Ok(false),
        }
    }

    /// One frame of BLOB (after the display work).
    pub fn blob_control(&mut self, input: &Input) -> Outcome {
        self.pad = input.pad.meaning;
        self.entities[5].0[COLOUR] = 7;
        self.set_blob(b::FIRE, 0);
        if input.keyboard(0xFD) & 0x1F == 0 {
            return Outcome::Quit;
        }
        if self.controls.pause_pressed(input) {
            return Outcome::Pause;
        }
        let ctl = self.controls.read(input);
        if ctl & 0x10 != 0 {
            self.set_blob(b::FIRE, self.blob(b::FIRE) + 1);
        }
        let mut c = ctl & 0x0F;
        if !c & 3 == 0 {
            c &= 0xFC;
        }
        if !c & 0x0C == 0 {
            c &= 0xF3;
        }
        if !c & 9 == 0 {
            c |= 0x40;
        }
        if !c & 0x0A == 0 {
            c |= 0x80;
        }
        if !c & 5 == 0 {
            c |= 0x20;
        }
        if !c & 6 == 0 {
            c |= 0x10;
        }
        self.set_blob(b::INPUT, c);
        if c != 0 {
            self.set_blob(b::LAST_INPUT, c);
            if c & 3 != 0 {
                self.set_blob(b::SHOT_FACING, c & 3);
            }
        }

        match self.blob(b::STATE) {
            LIFTED => {
                self.lifted();
                self.straight_shot();
            }
            HOVERING => {
                self.hovering();
                self.bouncing_shot();
                self.set_blob(b::INPUT, self.blob(b::INPUT) & !8);
                let y = self.blob(Y);
                if y < 0x16 {
                    self.set_blob(Y, 0x8F);
                    return self.change_room(16);
                }
                if y >= 0x90 {
                    self.set_blob(Y, 0x17);
                    return self.change_room(-16);
                }
            }
            _ => {
                self.walking();
                self.straight_shot();
            }
        }

        if let Some(outcome) = self.room_exit() {
            return outcome;
        }
        self.touch()
    }
}
