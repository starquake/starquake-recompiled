//! A USB gamepad, read as the Kempston joystick.
//!
//! A Kempston interface is a joystick port: the game reads five bits and
//! cannot tell what moved them, so a gamepad drives them exactly as the
//! hardware would. Pause is the odd one out — on a Spectrum it is a key, not
//! a joystick button — so Start presses `P` for convenience.

/// How far a stick must move before it counts as a direction.
const DEADZONE: f32 = 0.5;

pub struct Gamepad {
    gilrs: Option<gilrs::Gilrs>,
}

impl Gamepad {
    pub fn new() -> Gamepad {
        match gilrs::Gilrs::new() {
            Ok(gilrs) => Gamepad { gilrs: Some(gilrs) },
            Err(e) => {
                eprintln!("no gamepad support: {e}");
                Gamepad { gilrs: None }
            }
        }
    }

    /// The Kempston bits every connected pad is asking for, and whether one
    /// of them is asking to pause.
    pub fn poll(&mut self) -> (u8, bool) {
        let Some(gilrs) = &mut self.gilrs else {
            return (0, false);
        };
        // Reading the state is what the events feed, so drain them first;
        // this is also where hot-plugged pads arrive.
        while gilrs.next_event().is_some() {}

        let (mut bits, mut pause) = (0u8, false);
        for (_id, pad) in gilrs.gamepads() {
            use gilrs::{Axis, Button};
            let (x, y) = (pad.value(Axis::LeftStickX), pad.value(Axis::LeftStickY));
            if pad.is_pressed(Button::DPadRight) || x > DEADZONE {
                bits |= 0x01;
            }
            if pad.is_pressed(Button::DPadLeft) || x < -DEADZONE {
                bits |= 0x02;
            }
            if pad.is_pressed(Button::DPadDown) || y < -DEADZONE {
                bits |= 0x04;
            }
            if pad.is_pressed(Button::DPadUp) || y > DEADZONE {
                bits |= 0x08;
            }
            // Any of the buttons under a thumb or finger fires.
            let fire = [
                Button::South,
                Button::East,
                Button::North,
                Button::West,
                Button::RightTrigger,
                Button::LeftTrigger,
                Button::RightTrigger2,
                Button::LeftTrigger2,
            ];
            if fire.iter().any(|&b| pad.is_pressed(b)) {
                bits |= 0x10;
            }
            if pad.is_pressed(Button::Start) || pad.is_pressed(Button::Select) {
                pause = true;
            }
        }
        (bits, pause)
    }
}
