//! A gamepad, read as the Kempston joystick.
//!
//! A Kempston interface is a joystick port: the game reads five bits and
//! cannot tell what moved them, so a gamepad drives them exactly as the
//! hardware would. Pause is the odd one out — on a Spectrum it is a key, not
//! a joystick button — so Start presses `P` for convenience. Select opens
//! the guidance picker (#1), where the D-pad works it, A confirms and B
//! or Select cancels.
//!
//! How the pad is attached is not this code's business, or `gilrs`'s. A
//! Bluetooth controller the operating system has paired is an ordinary
//! gamepad by the time it reaches here, exactly as a USB one is; both arrive
//! through the same platform API. Hot-plugging is handled either way, since
//! `poll` drains the event queue before reading, which is where a pad that
//! has just connected turns up.

/// How far a stick must move before it counts as a direction.
const DEADZONE: f32 = 0.5;

/// What the pads are asking for this frame.
#[derive(Clone, Copy, Debug, Default)]
pub struct Pad {
    /// The Kempston bits.
    pub bits: u8,
    /// Start is held: pause.
    pub start: bool,
    /// Pressed since the last poll, for the picker: each is one press, not
    /// a button held down.
    pub select: bool,
    pub up: bool,
    pub down: bool,
    pub left: bool,
    pub right: bool,
    /// The bottom face button (A on an Xbox pad), which does the picker's
    /// highlighted action.
    pub south: bool,
    /// The right face button (B on an Xbox pad), which cancels the picker.
    pub east: bool,
}

pub struct Gamepad {
    gilrs: Option<gilrs::Gilrs>,
    /// Whether Select, the four directions and the bottom face button were
    /// down at the last poll,
    /// to tell a press from a hold.
    was: [bool; 7],
}

impl Gamepad {
    pub fn new() -> Gamepad {
        match gilrs::Gilrs::new() {
            Ok(gilrs) => Gamepad {
                gilrs: Some(gilrs),
                was: [false; 7],
            },
            Err(e) => {
                eprintln!("no gamepad support: {e}");
                Gamepad {
                    gilrs: None,
                    was: [false; 7],
                }
            }
        }
    }

    /// What every connected pad together is asking for.
    pub fn poll(&mut self) -> Pad {
        let Some(gilrs) = &mut self.gilrs else {
            return Pad::default();
        };
        // Reading the state is what the events feed, so drain them first;
        // this is also where hot-plugged pads arrive.
        while gilrs.next_event().is_some() {}

        let (mut bits, mut start) = (0u8, false);
        let mut now = [false; 7];
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
            start |= pad.is_pressed(Button::Start);
            now[0] |= pad.is_pressed(Button::Select);
            now[1] |= pad.is_pressed(Button::DPadUp) || y > DEADZONE;
            now[2] |= pad.is_pressed(Button::DPadDown) || y < -DEADZONE;
            now[3] |= pad.is_pressed(Button::DPadLeft) || x < -DEADZONE;
            now[4] |= pad.is_pressed(Button::DPadRight) || x > DEADZONE;
            now[5] |= pad.is_pressed(Button::South);
            now[6] |= pad.is_pressed(Button::East);
        }
        let pressed = |i: usize| now[i] && !self.was[i];
        let result = Pad {
            bits,
            start,
            select: pressed(0),
            up: pressed(1),
            down: pressed(2),
            left: pressed(3),
            right: pressed(4),
            south: pressed(5),
            east: pressed(6),
        };
        self.was = now;
        result
    }
}
