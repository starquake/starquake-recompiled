//! A gamepad, read as the Kempston joystick.
//!
//! A Kempston interface is a joystick port: the game reads five bits and
//! cannot tell what moved them, so a gamepad drives them exactly as the
//! hardware would. The D-pad and the left stick move left and right, and up
//! and down only on the hover platform; the bottom face button is down
//! (a platform), the right one up (an item), and the left one fires, as
//! platformers lay them out (#88); the top one does nothing in play. Pause is the odd one out — on a Spectrum
//! it is a key, not a joystick button — so Start presses the pause key.
//! Select opens the guidance picker (#1), where the D-pad works it, A does
//! an action wherever the pad's maker puts A, and B or Select closes it.
//!
//! How the pad is attached is not this code's business, or `gilrs`'s. A
//! Bluetooth controller the operating system has paired is an ordinary
//! gamepad by the time it reaches here, exactly as a USB one is; both arrive
//! through the same platform API. Hot-plugging is handled either way, since
//! `poll` drains the event queue before reading, which is where a pad that
//! has just connected turns up.

/// Which letters a pad's face buttons carry, from the maker it reports
/// itself as (#88). The buttons are read by position — `gilrs` names them
/// South, East, West and North whatever is printed on them — so this
/// changes only the letters shown in a legend, and which button confirms.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub enum Layout {
    /// A on the bottom, B right, X left, Y top. The default: what most pads
    /// for a computer are printed with, and what an unrecognised pad is
    /// taken to be.
    #[default]
    Xbox,
    /// A right, B bottom, X top, Y left.
    Nintendo,
    /// The cross at the bottom, the circle right, the square left, the
    /// triangle on top.
    PlayStation,
}

/// Nintendo's USB vendor: a Pro Controller, Joy-Cons, or a third-party pad
/// in its Nintendo mode, which reports itself as one.
const NINTENDO: u16 = 0x057E;
/// Sony's.
const PLAYSTATION: u16 = 0x054C;

impl Layout {
    /// The layout a pad reporting `vendor` carries.
    pub fn of(vendor: Option<u16>) -> Layout {
        match vendor {
            Some(NINTENDO) => Layout::Nintendo,
            Some(PLAYSTATION) => Layout::PlayStation,
            _ => Layout::Xbox,
        }
    }

    /// Whether the button that confirms is the right-hand one rather than
    /// the bottom one: A is on the right of a Nintendo pad, and A confirms.
    /// The cross confirms on a PlayStation pad, at the bottom as on an Xbox
    /// one.
    pub fn confirms_east(self) -> bool {
        self == Layout::Nintendo
    }
}

/// How far a stick must move before it counts as a direction.
const DEADZONE: f32 = 0.5;

/// What the pads are asking for this frame.
#[derive(Clone, Copy, Debug, Default)]
pub struct Pad {
    /// The Kempston bits: the D-pad's and the stick's (`dirs`) and the face
    /// buttons' together.
    pub bits: u8,
    /// The Kempston bits of the D-pad and the stick alone, so that up and
    /// down on them can be left out while BLOB walks (#88), and of the face
    /// buttons alone.
    pub dirs: u8,
    pub buttons: u8,
    /// Start is held: pause.
    pub start: bool,
    /// Pressed since the last poll, for the picker: each is one press, not
    /// a button held down.
    pub select: bool,
    pub up: bool,
    pub down: bool,
    pub left: bool,
    pub right: bool,
    /// The bottom face button (A on an Xbox pad).
    pub south: bool,
    /// The right face button (B on an Xbox pad).
    pub east: bool,
    /// The letters the first connected pad carries.
    pub layout: Layout,
}

impl Pad {
    /// The press that does the picker's highlighted action: A, which is the
    /// bottom button, or the right one on a pad whose A is there.
    pub fn confirm(&self) -> bool {
        if self.layout.confirms_east() {
            self.east
        } else {
            self.south
        }
    }

    /// The press that goes back: B, the other of the two.
    pub fn cancel(&self) -> bool {
        if self.layout.confirms_east() {
            self.south
        } else {
            self.east
        }
    }
}

pub struct Gamepad {
    gilrs: Option<gilrs::Gilrs>,
    /// Whether Select, the four directions and the bottom face button were
    /// down at the last poll,
    /// to tell a press from a hold.
    was: [bool; 7],
    /// Joystick bits, and Start, kept from the game until they are let go:
    /// what was held as the picker closed (#88).
    held_back: u8,
    start_held_back: bool,
    /// Whether the next poll starts holding back whatever is down.
    hold_back_next: bool,
}

impl Gamepad {
    pub fn new() -> Gamepad {
        match gilrs::Gilrs::new() {
            Ok(gilrs) => Gamepad {
                gilrs: Some(gilrs),
                ..Gamepad::none()
            },
            Err(e) => {
                eprintln!("no gamepad support: {e}");
                Gamepad::none()
            }
        }
    }

    /// No pad support at all.
    fn none() -> Gamepad {
        Gamepad {
            gilrs: None,
            was: [false; 7],
            held_back: 0,
            start_held_back: false,
            hold_back_next: false,
        }
    }

    /// From the next poll, keeps whatever is held then from the game until
    /// each is let go: the button that closed the picker is not also a
    /// platform, a shot or a pause (#88), as ZX Sidekick has it
    /// (zx-sidekick/zx-sidekick#25).
    pub fn hold_back_held(&mut self) {
        self.hold_back_next = true;
    }

    fn hold_back(&mut self, pad: &mut Pad) {
        if std::mem::take(&mut self.hold_back_next) {
            self.held_back = pad.bits;
            self.start_held_back = pad.start;
        }
        self.held_back &= pad.bits;
        self.start_held_back &= pad.start;
        pad.bits &= !self.held_back;
        pad.dirs &= !self.held_back;
        pad.buttons &= !self.held_back;
        pad.start &= !self.start_held_back;
    }

    /// What every connected pad together is asking for.
    pub fn poll(&mut self) -> Pad {
        let Some(gilrs) = &mut self.gilrs else {
            return Pad::default();
        };
        // Reading the state is what the events feed, so drain them first;
        // this is also where hot-plugged pads arrive.
        while gilrs.next_event().is_some() {}

        let (mut dirs, mut buttons, mut start) = (0u8, 0u8, false);
        let mut now = [false; 7];
        // The first pad listed decides the letters; the rest are read for
        // what they are pressing.
        let layout = gilrs
            .gamepads()
            .next()
            .map_or(Layout::Xbox, |(_, first)| Layout::of(first.vendor_id()));
        for (_id, pad) in gilrs.gamepads() {
            use gilrs::{Axis, Button};
            let (x, y) = (pad.value(Axis::LeftStickX), pad.value(Axis::LeftStickY));
            if pad.is_pressed(Button::DPadRight) || x > DEADZONE {
                dirs |= 0x01;
            }
            if pad.is_pressed(Button::DPadLeft) || x < -DEADZONE {
                dirs |= 0x02;
            }
            if pad.is_pressed(Button::DPadDown) || y < -DEADZONE {
                dirs |= 0x04;
            }
            if pad.is_pressed(Button::DPadUp) || y > DEADZONE {
                dirs |= 0x08;
            }
            // The bottom face button is down, which builds a platform, the
            // right one up, which picks up or swaps an item and boards the
            // hover platform, and the left one fires, as a platformer lays
            // them out (Shovel Knight's pad, for one). The top one does
            // nothing in play.
            if pad.is_pressed(Button::South) {
                buttons |= 0x04;
            }
            if pad.is_pressed(Button::East) {
                buttons |= 0x08;
            }
            if pad.is_pressed(Button::West) {
                buttons |= 0x10;
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
        let mut result = Pad {
            bits: dirs | buttons,
            dirs,
            buttons,
            start,
            select: pressed(0),
            up: pressed(1),
            down: pressed(2),
            left: pressed(3),
            right: pressed(4),
            south: pressed(5),
            east: pressed(6),
            layout,
        };
        self.was = now;
        self.hold_back(&mut result);
        result
    }
}

#[cfg(test)]
mod tests {
    use super::{Gamepad, Layout, Pad};

    #[test]
    fn the_maker_decides_the_letters() {
        assert_eq!(Layout::of(Some(0x057E)), Layout::Nintendo);
        assert_eq!(Layout::of(Some(0x054C)), Layout::PlayStation);
        assert_eq!(Layout::of(Some(0x045E)), Layout::Xbox, "Microsoft's");
        assert_eq!(Layout::of(None), Layout::Xbox, "unknown: Xbox letters");
    }

    #[test]
    fn what_closed_the_picker_is_kept_from_the_game_until_let_go() {
        let mut pad = Gamepad::none();
        let held = |bits: u8| Pad {
            bits,
            ..Pad::default()
        };
        pad.hold_back_held();
        let mut down = held(0x04);
        pad.hold_back(&mut down);
        assert_eq!(down.bits, 0, "the bottom button that closed it");
        let mut down_and_left = held(0x04 | 0x02);
        pad.hold_back(&mut down_and_left);
        assert_eq!(down_and_left.bits, 0x02, "a new press gets through");
        let mut let_go = held(0);
        pad.hold_back(&mut let_go);
        let mut down_again = held(0x04);
        pad.hold_back(&mut down_again);
        assert_eq!(down_again.bits, 0x04, "pressed again after letting go");
    }

    #[test]
    fn a_confirms_wherever_it_is() {
        let bottom = Pad {
            south: true,
            ..Pad::default()
        };
        let right = Pad {
            east: true,
            ..Pad::default()
        };
        for layout in [Layout::Xbox, Layout::PlayStation] {
            assert!(Pad { layout, ..bottom }.confirm(), "{layout:?}");
            assert!(Pad { layout, ..right }.cancel(), "{layout:?}");
        }
        let nintendo = Layout::Nintendo;
        assert!(
            Pad {
                layout: nintendo,
                ..right
            }
            .confirm()
        );
        assert!(
            Pad {
                layout: nintendo,
                ..bottom
            }
            .cancel()
        );
        assert!(
            !Pad {
                layout: nintendo,
                ..bottom
            }
            .confirm()
        );
    }
}
