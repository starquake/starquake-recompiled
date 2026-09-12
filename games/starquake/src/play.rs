//! The main loop: one call per 50 Hz frame of play.

use crate::blob::{Modal, Outcome};
use crate::controls::Input;
use crate::game::Game;
use crate::host::Host;

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum FrameEvent {
    Continue,
    /// BLOB died (death reason).
    Died(u8),
    Modal(Modal),
    /// Walked into the core room, which the caller runs.
    CoreRoom,
    Quit,
    Pause,
}

/// Death reasons.
pub mod death {
    pub const FORCE_FIELD: u8 = 0;
    pub const OUT_OF_ENERGY: u8 = 2;
}

fn distance(a: u8, b: u8) -> u8 {
    if a < b { b - a } else { a - b }
}

impl Game {
    /// The display work done at the start of every frame, after the frame
    /// boundary.
    pub fn display_work(&mut self) {
        self.draw_sprites();
        self.colour_sprites();
        self.tick_platforms();
        self.rng.step();
        self.tick_sparkles();
        self.tick_force_fields();
    }

    /// The sound tick and display work, without a host (for verification).
    pub fn frame_display(&mut self) {
        self.tone = self.sound_tick();
        self.display_work();
    }

    /// The sound tick, the frame boundary, and the display work.
    pub fn frame_with(&mut self, host: &mut dyn Host) {
        let tone = self.sound_tick();
        self.tone = tone;
        self.sync(host);
        self.tone = tone;
        self.display_work();
    }

    /// Whether BLOB is inside an active force field.
    fn force_field_contact(&self) -> bool {
        let (bx, by) = (self.entities[0].x(), self.entities[0].y());
        let rec = &self.objects.force_fields;
        for base in (0..).step_by(8) {
            let col = rec.get(base).copied().unwrap_or(0);
            if col == 0 {
                return false;
            }
            if distance(col.rotate_left(3), bx) >= 0x0E {
                continue;
            }
            let top = 0x1Au8.wrapping_sub(rec[base + 1]).rotate_left(3).wrapping_sub(2);
            if top >= by && top - by < 0x17 && rec[base + 5] != 0 {
                return true;
            }
        }
        unreachable!()
    }

    /// Everything in a frame of play after the display work.
    pub fn play_logic(&mut self, input: &Input) -> FrameEvent {
        match self.blob_control(input) {
            Outcome::Continue => {}
            Outcome::NewRoom(reason) => {
                self.entry_reason = reason;
                self.enter_room();
                if self.room == crate::cores::CORE_ROOM {
                    return FrameEvent::CoreRoom;
                }
                return FrameEvent::Continue;
            }
            Outcome::Died(reason) => return FrameEvent::Died(reason),
            Outcome::Modal(m) => return FrameEvent::Modal(m),
            Outcome::Quit => return FrameEvent::Quit,
            Outcome::Pause => return FrameEvent::Pause,
        }
        if self.status.bars[0] == 0 {
            return FrameEvent::Died(death::OUT_OF_ENERGY);
        }
        if self.force_field_contact() {
            return FrameEvent::Died(death::FORCE_FIELD);
        }
        match self.update_enemies() {
            Some(reason) => FrameEvent::Died(reason),
            None => FrameEvent::Continue,
        }
    }

    /// Runs one frame of play (without a host, for verification).
    pub fn play_frame(&mut self, input: &Input) -> FrameEvent {
        self.frame_display();
        self.play_logic(input)
    }

    /// Plays from entering the current room until the game ends.
    pub fn play(&mut self, host: &mut dyn Host) {
        self.enter_room();
        loop {
            self.frame_with(host);
            let input = self.input;
            let mut event = self.play_logic(&input);
            if event == FrameEvent::Pause {
                // Wait for the pause key to be let go, then for any control.
                while self.controls.pause_pressed(&self.input) {
                    self.sync(host);
                }
                while self.controls.read(&self.input) == 0 {
                    self.sync(host);
                }
                let input = self.input;
                event = self.play_logic(&input);
            }
            match event {
                FrameEvent::Continue | FrameEvent::Pause => {}
                FrameEvent::Died(reason) => {
                    if !self.death_sequence(reason, host) {
                        return;
                    }
                    self.enter_room();
                }
                FrameEvent::Modal(m) => {
                    self.entry_reason = self.run_modal(m, host);
                    self.enter_room();
                }
                FrameEvent::CoreRoom => {
                    if self.core_room(host) {
                        return;
                    }
                    self.enter_room();
                }
                FrameEvent::Quit => {
                    self.final_scoring();
                    return;
                }
            }
        }
    }
}
