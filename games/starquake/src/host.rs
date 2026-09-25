//! The interface between the game and whatever presents it.
//!
//! The game runs as ordinary sequential code (menus, the play loop, the
//! death sequence…). Wherever the original waits for the next 50 Hz
//! interrupt, the rewrite calls [`Game::sync`], which hands the frame to the
//! [`Host`]: show the display, play the sound, and report the input.

use crate::controls::Input;
use crate::game::Game;

pub trait Host {
    /// A frame boundary. Present `game.display`; play the frame's sound as
    /// [`Game::frame_sound`] lays it out (the blocking effects requested
    /// since the last boundary, and the tone); return the input for the next
    /// frame and how many 50 Hz frames passed (more than one if the sound
    /// overran).
    fn frame(&mut self, game: &Game) -> (Input, u32);

    /// The CORE OF HEROES table is about to be shown after a game: `table`
    /// as the game keeps it (eight entries of ten bytes: three initials,
    /// six score digits, the percentage), and `new`, the entry this game
    /// put in, if it made one. A host that keeps the table between runs
    /// saves it here (#90). Nothing by default.
    fn heroes(&mut self, _table: &[u8], _new: Option<usize>) {}

    /// The table has been shown: a table to put back in place of the game's,
    /// or `None` to leave it. A host puts back the one from before a game
    /// that is not to be kept, such as one played with training mode.
    fn heroes_shown(&mut self) -> Option<Vec<u8>> {
        None
    }

    /// Training mode's switches for the frame to come (#4). All off by
    /// default, which is the original game.
    fn training(&mut self) -> crate::game::Training {
        crate::game::Training::default()
    }
}

/// A host that shows and plays nothing and always reports the same input.
#[derive(Default)]
pub struct NullHost {
    pub input: Input,
    pub frames: u64,
}

impl Host for NullHost {
    fn frame(&mut self, game: &Game) -> (Input, u32) {
        let frames = game.frame_sound().frames;
        self.frames += frames as u64;
        (self.input, frames)
    }
}

pub use zx_core::timing::FRAMES_PER_SECOND;

/// Spreads a loop that the original paces by its own speed over the frames
/// the host gives us.
///
/// The original's menu loops are not driven by the interrupt: they go round
/// as fast as they can redraw themselves, about 13 times a second for the
/// title menu and about 344 for the define-keys loop, both measured from the
/// original (`sq-verify menu`). One is slower than the frame rate and one
/// much faster, so the same helper has to serve both.
///
/// The accumulator keeps the long-run rate exact and never grows; counting
/// frames and multiplying overflows after about eleven weeks on the menu.
pub struct Pacer {
    per_second: u32,
    acc: u32,
}

impl Pacer {
    pub fn new(per_second: u32) -> Pacer {
        Pacer { per_second, acc: 0 }
    }

    /// How many turns of the loop belong to one frame. Less than one most
    /// frames for a slow loop, several for a fast one.
    pub fn turns(&mut self) -> u32 {
        self.acc += self.per_second;
        let turns = self.acc / FRAMES_PER_SECOND;
        self.acc %= FRAMES_PER_SECOND;
        turns
    }
}

impl Game {
    /// Waits for the next frame.
    pub fn sync(&mut self, host: &mut dyn Host) {
        let (input, frames) = host.frame(self);
        self.effects.clear();
        self.effect_pictures.clear();
        self.music.clear();
        self.tone = None;
        self.play_work = false;
        self.work = crate::sound::Work::default();
        self.work_at_effect = None;
        self.input = input;
        self.frames = self.frames.wrapping_add(frames) & 0xFF_FFFF;
        let training = host.training();
        // A Full switch fills its bar as it goes on, in play, where the
        // status panel shows it.
        if self.scene == crate::game::Scene::Play {
            let filled = (0..3).filter(|&i| training.full(i) && !self.training.full(i));
            let mut drawn = false;
            for i in filled {
                self.status.bars[i] = 0x7F;
                drawn = true;
            }
            if drawn {
                self.draw_status();
            }
        }
        self.training = training;
    }

    /// Waits for `n` frames with nothing happening (the original's HALT loops).
    pub fn pause_frames(&mut self, host: &mut dyn Host, n: u32) {
        for _ in 0..n {
            self.sync(host);
        }
    }
}
