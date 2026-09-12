//! The interface between the game and whatever presents it.
//!
//! The game runs as ordinary sequential code (menus, the play loop, the
//! death sequence…). Wherever the original waits for the next 50 Hz
//! interrupt, the rewrite calls [`Game::sync`], which hands the frame to the
//! [`Host`]: show the display, play the sound, and report the input.

use crate::controls::Input;
use crate::game::Game;

pub trait Host {
    /// A frame boundary. Present `game.display`; play `game.effects` (the
    /// blocking sound effects requested since the last boundary) followed by
    /// `game.tone` for the rest of the frame; return the input for the next
    /// frame and how many 50 Hz frames passed (more than one if the sound
    /// effects overran).
    fn frame(&mut self, game: &Game) -> (Input, u32);
}

/// A host that shows and plays nothing and always reports the same input.
#[derive(Default)]
pub struct NullHost {
    pub input: Input,
    pub frames: u64,
}

impl Host for NullHost {
    fn frame(&mut self, game: &Game) -> (Input, u32) {
        let busy: u32 = game.effects.iter().map(|&id| crate::sound::beep(&game.assets.ram, id).1).sum();
        let frames = 1 + busy / crate::sound::FRAME_T;
        self.frames += frames as u64;
        (self.input, frames)
    }
}

impl Game {
    /// Waits for the next frame.
    pub fn sync(&mut self, host: &mut dyn Host) {
        let (input, frames) = host.frame(self);
        self.effects.clear();
        self.music.clear();
        self.tone = None;
        self.input = input;
        self.frames = self.frames.wrapping_add(frames) & 0xFF_FFFF;
    }

    /// Waits for `n` frames with nothing happening (the original's HALT loops).
    pub fn pause_frames(&mut self, host: &mut dyn Host, n: u32) {
        for _ in 0..n {
            self.sync(host);
        }
    }
}
