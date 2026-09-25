//! The per-frame sound tick, the blocking effects, and where a frame's sound
//! falls in time.
//!
//! Two "channels" request effects (`sound[0]` for BLOB's actions,
//! `sound[1]` for enemies and ambience); the tick loads the requested
//! effect's parameters and each frame advances its pitch sweep. The
//! original then toggles the beeper at the resulting pitch until the next
//! interrupt, which is also what paces the game to 50 Hz. The frame's work
//! comes after the interrupt, in silence, so what a player hears is silence
//! and then tone, fifty times a second.
//!
//! Layout of `sound` (as in the original): request 1, request 2, frames
//! left, pitch, pitch step, wobble, prescaler, prescaler reload.

use zx_core::bus::{Cycle, Kind, charge, charge_io};

use crate::game::Game;

/// Effect parameter table in the original (4 bytes per effect).
const EFFECTS: usize = 0xA607;
/// Blocking sound effect parameters (5 bytes per effect).
const BEEPS: usize = 0xD839;

/// T-states per 50 Hz frame.
pub use zx_core::timing::FRAME_T;

/// How long the ROM's 50 Hz interrupt routine holds up whatever it breaks
/// into, the acknowledge included: counting the frame, and scanning a
/// keyboard with nothing held. Measured in the reference interpreter; it is
/// the same at every frame boundary.
pub const INTERRUPT_T: u32 = 895;

/// The clock the sound routines run on: T-states from a frame boundary,
/// with the ULA's delays and the 50 Hz interrupt.
///
/// The original's sound is a loop that toggles the speaker and counts, so
/// its pitch is however long the loop takes. That depends on when it runs:
/// its stack is in the memory the ULA shares (the tape's loader puts it
/// there with `CLEAR 24103`), and so is the frame counter the tone loop
/// watches, so while the picture is being drawn every one of those accesses
/// waits, and the same effect plays lower than it does in the border. Each
/// instruction is therefore charged cycle by cycle, through `zx_core::bus`
/// like the reference interpreter's.
struct Clock {
    t: u32,
    /// The next frame boundary, where the interrupt is taken at the start
    /// of the first instruction after it.
    next_interrupt: u32,
}

impl Clock {
    /// Where the stack is. All of it is contended, so one address stands for
    /// every byte of it.
    const STACK: u16 = 0x5DE6;
    /// The system variable the tone loop polls to see a frame go by.
    const FRAMES: u16 = 0x5C78;

    fn at(t: u32) -> Clock {
        Clock {
            t,
            next_interrupt: (t / FRAME_T + 1) * FRAME_T,
        }
    }

    /// Starts an instruction, taking the interrupt first if a frame boundary
    /// has gone by. Returns whether it did.
    fn instruction(&mut self) -> bool {
        if self.t < self.next_interrupt {
            return false;
        }
        self.t += INTERRUPT_T;
        self.next_interrupt += FRAME_T;
        true
    }

    /// Instructions that touch only the processor and memory above 0x8000,
    /// which cost what the manual says whenever they run.
    fn plain(&mut self, costs: &[u32]) {
        for &c in costs {
            self.instruction();
            self.t += c;
        }
    }

    fn stack(&mut self, kind: Kind) {
        charge(
            &mut self.t,
            Cycle {
                at: Self::STACK,
                len: 3,
                kind,
            },
        );
    }

    /// `PUSH rr` (`prefix` 4 for `PUSH IX`): the opcode, a cycle on the
    /// refresh address, then two bytes written to the stack.
    fn push(&mut self, prefix: u32) {
        self.instruction();
        self.t += prefix + 5;
        self.stack(Kind::Write);
        self.stack(Kind::Write);
    }

    /// `POP rr` (`prefix` 4 for `POP IX`), and `RET`.
    fn pop(&mut self, prefix: u32) {
        self.instruction();
        self.t += prefix + 4;
        self.stack(Kind::Read);
        self.stack(Kind::Read);
    }

    /// `CALL nn` from the game's own code.
    fn call(&mut self) {
        self.instruction();
        self.t += 4 + 3 + 4;
        self.stack(Kind::Write);
        self.stack(Kind::Write);
    }

    /// `OUT (n),A` with A the speaker byte: returns the T-state at which the
    /// instruction ends, which is when the speaker is taken to change.
    fn out(&mut self, a: u8) -> u32 {
        self.instruction();
        self.t += 4 + 3;
        charge_io(&mut self.t, u16::from(a) << 8 | 0x00FE);
        self.t
    }
}

/// Speaker changes of a blocking sound effect started at `start` (T-states
/// from a frame boundary, counting from the `CALL`), as (T-state, level)
/// pairs on the same clock, and the T-state at which it has returned.
/// Follows the original routine (`D7C0`) instruction by instruction, so the
/// pitch and length match wherever in the frame it plays. Ids the game
/// never asks for are silent and instant.
///
/// # Panics
///
/// If the loaded game data is too short to hold what the original keeps
/// there, which means the file was not Starquake.
pub fn beep(ram: &[u8], id: u8, start: u32) -> (Vec<(u32, bool)>, u32) {
    if id as usize >= crate::assets::EFFECT_COUNT {
        return (Vec::new(), start);
    }
    let p = BEEPS + id.wrapping_mul(5) as usize;
    let ix: [u8; 5] = ram[p..p + 5].try_into().unwrap();
    let mut out = Vec::new();
    let mut c = Clock::at(start);
    c.call();
    // push hl, bc, de, ix
    c.push(0);
    c.push(0);
    c.push(0);
    c.push(4);
    // ld d,0; ld e,a; rlca; rlca; add e; ld e,a; ld hl,D839; add hl,de
    c.plain(&[7, 4, 4, 4, 4, 4, 10, 11]);
    c.push(0);
    c.pop(4);
    // ld a,(ix+4); and 1f
    c.plain(&[19, 7]);
    let mut count = ix[4] & 0x1F;
    loop {
        c.push(0);
        // ld h,(ix+0); ld l,(ix+2); bit 6,(ix+4)
        c.plain(&[19, 19, 20]);
        let h0 = ix[0];
        let mut l = ix[2];
        if ix[4] & 0x40 != 0 {
            // jr z (not taken); ld e,a; ld a,l; bit 5,(ix+4)
            c.plain(&[7, 4, 4, 20]);
            l = if ix[4] & 0x20 != 0 {
                // jr z (not taken); sub e; jr
                c.plain(&[7, 4, 12]);
                l.wrapping_sub(count)
            } else {
                // jr z; add e
                c.plain(&[12, 4]);
                l.wrapping_add(count)
            };
            // cp 0; ld l,a
            c.plain(&[7, 4]);
            if l == 0 {
                // jr nz (not taken); inc l
                c.plain(&[7, 4]);
                l = 1;
            } else {
                c.plain(&[12]);
            }
        } else {
            c.plain(&[12]);
        }
        // ld c,0
        c.plain(&[7]);
        let mut speaker = 0u8;
        let mut h = h0;
        loop {
            // ld b,h
            c.plain(&[4]);
            let mut b = h;
            loop {
                // ld a,c; out (fe),a
                c.plain(&[4]);
                let at = c.out(speaker);
                out.push((at, speaker != 0));
                speaker ^= 0x10;
                // xor 10; ld c,a; ld a,h; and b; xor (ix+3); ld d,a; bit 7,(ix+4)
                c.plain(&[7, 4, 4, 4, 19, 4, 20]);
                let mut d = (h & b) ^ ix[3];
                if ix[4] & 0x80 != 0 {
                    // jr z (not taken); ld a,d; srl a; sub h; and 3f; ld d,a
                    c.plain(&[7, 4, 8, 4, 7, 4]);
                    d = (d >> 1).wrapping_sub(h) & 0x3F;
                } else {
                    c.plain(&[12]);
                }
                // The delay: push ix; pop ix; dec d; jr nz. The push and pop
                // are the whole reason the pitch depends on the picture.
                loop {
                    c.push(4);
                    c.pop(4);
                    d = d.wrapping_sub(1);
                    if d == 0 {
                        c.plain(&[4, 7]);
                        break;
                    }
                    c.plain(&[4, 12]);
                }
                // ld a,b; sub l
                c.plain(&[4, 4]);
                let (next, borrow) = b.overflowing_sub(l);
                if borrow {
                    c.plain(&[12]);
                    break;
                }
                // jr c (not taken); ld b,a; jr
                c.plain(&[7, 4, 12]);
                b = next;
            }
            // ld a,h; cp (ix+1)
            c.plain(&[4, 19]);
            if h == ix[1] {
                c.plain(&[12]);
                break;
            }
            if h < ix[1] {
                // jr z (not taken); jr c; inc h; jr
                c.plain(&[7, 12, 4, 12]);
                h = h.wrapping_add(1);
            } else {
                // jr z, jr c (neither taken); dec h; jr
                c.plain(&[7, 7, 4, 12]);
                h = h.wrapping_sub(1);
            }
        }
        c.pop(0);
        // dec a
        c.plain(&[4]);
        count = count.wrapping_sub(1);
        if count == 0 {
            c.plain(&[7]);
            break;
        }
        c.plain(&[12]);
    }
    // pop ix, de, bc, hl; ret
    c.pop(4);
    c.pop(0);
    c.pop(0);
    c.pop(0);
    c.pop(0);
    (out, c.t)
}

/// Speaker changes of the frame tone, from the tone loop (`A5BA`) starting
/// at `start` until it sees the next frame boundary go by. The same clock
/// as [`beep`]: while the picture is drawn, every poll of the frame counter
/// waits on the ULA, so the note is lower in the first part of the frame
/// than in the border after it.
pub fn tone(half_period: u8, start: u32) -> Vec<(u32, bool)> {
    let mut out = Vec::new();
    let mut c = Clock::at(start);
    let boundary = c.next_interrupt;
    let mut speaker = 0u8;
    loop {
        // ld a,c; xor 10; ld c,a
        c.plain(&[4, 7, 4]);
        speaker ^= 0x10;
        let at = c.out(speaker);
        out.push((at, speaker != 0));
        // ld b,e
        c.plain(&[4]);
        let mut b = half_period;
        loop {
            // ld a,(5c78): once the interrupt has counted a frame, this
            // read sees it and the `ret nz` after it leaves.
            c.instruction();
            c.t += 4 + 3 + 3;
            charge(&mut c.t, Cycle::read(Clock::FRAMES));
            if c.next_interrupt != boundary {
                return out;
            }
            // cp d; ret nz (not taken); djnz
            c.plain(&[4, 5]);
            b = b.wrapping_sub(1);
            if b == 0 {
                c.plain(&[8]);
                break;
            }
            c.plain(&[13]);
        }
        // jr
        c.plain(&[12]);
    }
}

/// What a frame of play did that the original takes longer over.
///
/// In play the original spends the start of every frame on its work, in
/// silence, and only then plays the frame's tone; so how long the silence
/// lasts depends on the work. The rewrite does the same work without a
/// clock, so it counts the parts that vary and prices them. Each price is
/// what that part costs the original on average, fitted in the reference
/// interpreter over thousands of frames of play (`sq-verify`'s *sound work*
/// check fits them again and fails if these drift).
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub struct Work {
    /// Horizontal collision tests (`D2F0`), by BLOB and the enemies.
    pub collisions: u32,
    /// Vertical collision tests (`D2F4`).
    pub vertical_collisions: u32,
    /// Characters printed through the ROM.
    pub characters: u32,
    /// Character cells XORed onto the screen (`DB3B`).
    pub cells: u32,
    /// Enemies checked against BLOB's position (`A305`).
    pub proximity_checks: u32,
}

impl Work {
    /// The fitted prices, in the order of the fields.
    const PRICES: [u32; 5] = [1_574, 175, 2_742, 1_586, 264];

    /// The counts, in the order of the fields.
    #[must_use]
    pub fn counts(&self) -> [u32; 5] {
        [
            self.collisions,
            self.vertical_collisions,
            self.characters,
            self.cells,
            self.proximity_checks,
        ]
    }

    /// What the varying part of the work cost the original.
    #[must_use]
    pub fn t(&self) -> u32 {
        self.counts()
            .iter()
            .zip(Self::PRICES)
            .map(|(n, price)| n * price)
            .sum()
    }
}

/// When the tone loop starts in a frame of play whose work counted nothing,
/// in T-states from the frame boundary: the interrupt, then sprites,
/// colours, platforms, sparkles, force fields, BLOB and the enemies at their
/// plainest. Fitted with [`Work`]'s prices.
pub const WORK_T: u32 = 35_480;
/// When BLOB's first blocking effect starts in a frame whose work before it
/// counted nothing (it is BLOB who asks for them, after the display work).
pub const WORK_BEFORE_EFFECTS_T: u32 = 31_185;

/// A frame's sound, laid out in time.
pub struct FrameSound {
    /// Speaker changes, as (T-states from the frame boundary, level).
    pub edges: Vec<(u32, bool)>,
    /// How many 50 Hz frames went by: more than one when the effects ran
    /// past the boundary.
    pub frames: u32,
}

impl Game {
    /// Where this frame's sound goes.
    ///
    /// In play the original does the frame's work first, silently, then
    /// toggles the speaker for the tone until the next interrupt. So a frame
    /// with a tone is silence and then the tone, fifty times a second, and a
    /// blocking effect sits in the silence where BLOB asked for it. Elsewhere
    /// (menus, screens, deaths) the effects play from the boundary, followed
    /// by the tune if there is one.
    pub fn frame_sound(&self) -> FrameSound {
        let ram = &self.assets.ram;
        let mut edges = Vec::new();
        let before = WORK_BEFORE_EFFECTS_T + self.work_at_effect.unwrap_or_default().t();
        let work = WORK_T + self.work.t();
        let mut t = if self.play_work && !self.effects.is_empty() {
            before
        } else {
            0
        };
        for &id in &self.effects {
            let (e, end) = beep(ram, id, t);
            edges.extend(e);
            t = end;
        }
        if !self.music.is_empty() {
            let frames = 1 + t / FRAME_T;
            let base = t;
            edges.extend(
                self.music
                    .iter()
                    .map(|&(at, level)| (base + at, level))
                    .take_while(|&(at, _)| at < frames * FRAME_T),
            );
            return FrameSound { edges, frames };
        }
        let tone_at = match (self.play_work, self.effects.is_empty()) {
            (true, true) => work,
            (true, false) => {
                // The rest of the work after the effects.
                let at = t + work.saturating_sub(before);
                // An interrupt in that work holds it up as well.
                at + INTERRUPT_T * (at / FRAME_T - t / FRAME_T)
            }
            (false, _) => t,
        };
        if let Some(half_period) = self.tone {
            edges.extend(tone(half_period, tone_at));
        }
        FrameSound {
            edges,
            frames: 1 + tone_at / FRAME_T,
        }
    }

    /// Asks for a blocking sound effect, noting how far the frame's work had
    /// got if it is the first.
    pub fn request_effect(&mut self, id: u8) {
        if self.effects.is_empty() {
            self.work_at_effect = Some(self.work);
        }
        self.effects.push(id);
    }

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
