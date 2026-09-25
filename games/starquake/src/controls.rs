//! Input: the Spectrum keyboard matrix and Kempston joystick, and the
//! game's configurable controls.

/// The state of the input devices, as the original would read them.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct Input {
    /// Keyboard half-rows (address lines A8–A15); a 0 bit is a pressed key.
    pub keys: [u8; 8],
    /// Kempston joystick (bit 0 right, 1 left, 2 down, 3 up, 4 fire).
    pub kempston: u8,
    /// A gamepad, as its own input rather than as keys (#123). Not part of
    /// what the original reads: at rest, it is the original.
    pub pad: PadInput,
}

impl Default for Input {
    fn default() -> Self {
        Input {
            keys: [0xFF; 8],
            kempston: 0,
            pad: PadInput::default(),
        }
    }
}

/// A gamepad. It moves and fires whatever control method is chosen, pauses
/// with Start, starts a game from the title screen and ends a "press any
/// key" wait, but it never types: whatever the game reads as letters and
/// digits (initials, teleporter codes, the menu) hears the keyboard only.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub struct PadInput {
    /// Directions and fire, in the joystick's bits (1 right, 2 left, 4 down,
    /// 8 up, 0x10 fire).
    pub bits: u8,
    /// Start: pause in play, and start a game from the title screen.
    pub start: bool,
    /// Which of up's and down's meanings its press carries.
    pub meaning: PadMeaning,
}

impl PadInput {
    /// Whether anything on it is pressed: what ends a "press any key" wait.
    pub fn any(&self) -> bool {
        self.bits != 0 || self.start
    }
}

/// In the original, up means both picking up an item and boarding or
/// flying the hover platform, and down both building a platform and flying
/// down. A gamepad splits them (#112): the D-pad's up and down only board
/// and fly; the button that is up only picks up, the button that is down
/// only builds. The game obeys these where it decides each (`Game::touch`,
/// `Game::touch_marker`, `Game::hovering`, `Game::build_platform`). All
/// off, up and down mean both.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub struct PadMeaning {
    /// Up comes from the D-pad alone: it boards and flies, never picks up.
    pub up_moves_only: bool,
    /// Up comes from its button alone: it picks up, never boards or flies.
    pub up_picks_only: bool,
    /// Down comes from its button alone: it builds, never flies down.
    pub down_builds_only: bool,
    /// Down comes from the D-pad alone: it flies down, never builds.
    pub down_moves_only: bool,
}

impl Input {
    /// Holds down the key at `port`, the high address byte that selects its
    /// half-row, and `bit`.
    pub fn press_key(&mut self, (port, bit): (u8, u8)) {
        if let Some(row) = (0..8).find(|r| port & (1 << r) == 0) {
            self.keys[row] &= !(1 << bit);
        }
    }

    /// Keyboard bits for port `0xFE` with high address byte `hi`: rows
    /// whose address line is low are combined.
    pub fn keyboard(&self, hi: u8) -> u8 {
        let mut v = 0x1F;
        for (row, bits) in self.keys.iter().enumerate() {
            if hi & (1 << row) == 0 {
                v &= bits;
            }
        }
        v
    }
}

/// Which controls are in use. The original keeps these as operands inside
/// its input routine, which the menu rewrites.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct Controls {
    pub kempston: bool,
    /// Bits before any key is read (always 0 in practice).
    pub initial: u8,
    /// Keyboard controls: port high byte, bit, and direction bits to add,
    /// in the order left, down, up, right, fire.
    pub keys: [(u8, u8, u8); 5],
    /// Pause key: port high byte and bit.
    pub pause: (u8, u8),
}

/// Key names in keyboard-matrix order (5 per half-row, from `0xFE` to
/// `0x7F`), in the original.
const KEY_NAMES: usize = 0x62D3;
/// Key codes for typing, same order.
const KEY_CODES: usize = 0xD5A0;

/// The code of the one key held down, or 0 if none or several are
/// (letters and digits are ASCII, space 0x20, Enter 2, the shifts 1 and 3).
pub fn key_code(ram: &[u8], input: &Input) -> u8 {
    let mut code = 0;
    let mut held = 0;
    let mut port = 0xFEu8;
    for row in 0..8 {
        let bits = !input.keyboard(port) & 0x1F;
        for bit in 0..5 {
            if bits & (1 << bit) != 0 {
                code = ram[KEY_CODES + row * 5 + bit];
                held += 1;
            }
        }
        port = port.rotate_left(1);
    }
    if held == 1 { code } else { 0 }
}

/// Whether any key at all is held. Unlike [`key_code`] this does not care
/// how many are down; the music player tests the keyboard this way.
pub fn any_key(input: &Input) -> bool {
    input.keys.iter().any(|&row| row & 0x1F != 0x1F)
}

/// Finds the matrix position (port high byte, bit) of the key the game
/// calls `name`.
pub fn key_position(ram: &[u8], name: u8) -> Option<(u8, u8)> {
    let mut port = 0xFEu8;
    for row in 0..8 {
        for bit in 0..5 {
            if ram[KEY_NAMES + row * 5 + bit] == name {
                return Some((port, bit as u8));
            }
        }
        port = port.rotate_left(1);
    }
    None
}

/// Where the operands live in the original.
const KEMPSTON_FLAG: usize = 0xC567;
const INITIAL: usize = 0xC577;
const PORTS: [usize; 5] = [0xC57A, 0xC585, 0xC590, 0xC59B, 0xC5A6];
const BIT_OPCODES: [usize; 5] = [0xC57E, 0xC589, 0xC594, 0xC59F, 0xC5AA];
const VALUES: [usize; 5] = [0xC582, 0xC58D, 0xC598, 0xC5A3, 0xC5AD];
const PAUSE_PORT: usize = 0xC55C;
/// The operand byte of `bit n,a`, past the `CB` prefix — as with the keys
/// above, it is the second byte that carries the bit number.
const PAUSE_BIT_OPCODE: usize = 0xC560;

impl Controls {
    pub fn from_memory(mem: &[u8]) -> Controls {
        Controls {
            kempston: mem[KEMPSTON_FLAG] == 1,
            initial: mem[INITIAL],
            keys: std::array::from_fn(|i| {
                (
                    mem[PORTS[i]],
                    (mem[BIT_OPCODES[i]].wrapping_sub(0x42) >> 3) & 7,
                    mem[VALUES[i]],
                )
            }),
            pause: (
                mem[PAUSE_PORT],
                (mem[PAUSE_BIT_OPCODE].wrapping_sub(0x47) >> 3) & 7,
            ),
        }
    }

    /// Sets the keyboard controls from key names in the order the game's
    /// tables use: left, right, down, up, fire; then the pause key.
    /// Sets the five control keys and the pause key by name.
    ///
    /// # Panics
    ///
    /// If a name is not one the Spectrum's keyboard has. The names come from
    /// the original's own table, so only a caller inventing one can trip it.
    pub fn set_keys(&mut self, ram: &[u8], names: [u8; 5], pause: u8) {
        const ORDER: [usize; 5] = [0, 3, 1, 2, 4];
        for (&name, &slot) in names.iter().zip(&ORDER) {
            let (port, bit) = key_position(ram, name).expect("known key name");
            self.keys[slot].0 = port;
            self.keys[slot].1 = bit;
        }
        self.pause = key_position(ram, pause).expect("known key name");
    }

    /// The method's pause key, or a gamepad's Start.
    pub fn pause_pressed(&self, input: &Input) -> bool {
        input.keyboard(self.pause.0) & (1 << self.pause.1) == 0 || input.pad.start
    }

    /// Direction bits (1 right, 2 left, 4 down, 8 up) and fire (0x10): the
    /// method's own, and a gamepad's on top whatever the method.
    pub fn read(&self, input: &Input) -> u8 {
        self.read_method(input) | input.pad.bits
    }

    /// What the original's input routine reads for the chosen method.
    fn read_method(&self, input: &Input) -> u8 {
        if self.kempston && input.kempston != 0 {
            return input.kempston;
        }
        let pressed = |&(port, bit, _): &(u8, u8, u8)| input.keyboard(port) & (1 << bit) == 0;
        let mut v = self.initial;
        for key in &self.keys {
            if pressed(key) {
                v = v.wrapping_add(key.2);
            }
        }
        v
    }
}

impl crate::game::Game {
    /// Waits until no key is held, or until two or more are: what
    /// [`key_code`] reports as nothing; and until a gamepad is let go.
    ///
    /// Every screen that asks for a keypress does this first, so a key or
    /// button still down from the screen before is not read as the answer to
    /// this one.
    pub fn wait_keys_released(&mut self, host: &mut dyn crate::host::Host) {
        while key_code(&self.assets.ram, &self.input) != 0 || self.input.pad.any() {
            self.sync(host);
        }
    }

    /// Waits for a key `accept` likes and returns its code, running a frame
    /// between polls so the screen keeps moving.
    pub fn wait_key(
        &mut self,
        host: &mut dyn crate::host::Host,
        accept: impl Fn(u8) -> bool,
    ) -> u8 {
        loop {
            let k = key_code(&self.assets.ram, &self.input);
            if accept(k) {
                return k;
            }
            self.sync(host);
        }
    }

    /// [`wait_keys_released`](Self::wait_keys_released) then
    /// [`wait_key`](Self::wait_key): the pair almost
    /// every one of these screens wants.
    pub fn ask_key(&mut self, host: &mut dyn crate::host::Host, accept: impl Fn(u8) -> bool) -> u8 {
        self.wait_keys_released(host);
        self.wait_key(host, accept)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    /// Controls in the shape every method has: Kempston values on five
    /// distinct keys. The positions are made up, spread over several rows.
    fn keyboard() -> Controls {
        Controls {
            kempston: false,
            initial: 0,
            keys: [
                (0xFB, 0, 0x02),
                (0xFD, 2, 0x04),
                (0xFD, 3, 0x08),
                (0xDF, 1, 0x01),
                (0x7F, 4, 0x10),
            ],
            pause: (0x7F, 0),
        }
    }

    #[test]
    fn a_pad_moves_and_fires_in_every_kind_of_method() {
        for c in [
            keyboard(),
            Controls {
                kempston: true,
                ..keyboard()
            },
        ] {
            for bits in 0..0x20 {
                let mut input = Input::default();
                input.pad.bits = bits;
                assert_eq!(c.read(&input), bits, "bits {bits:#04x}");
            }
        }
    }

    #[test]
    fn a_pad_types_nothing() {
        let mut input = Input::default();
        input.pad.bits = 0x1F;
        input.pad.start = true;
        assert!(!any_key(&input), "no key is held");
        assert!(input.pad.any(), "but the pad is");
    }

    #[test]
    fn the_pad_adds_to_the_methods_own_keys() {
        let c = keyboard();
        let mut input = Input::default();
        input.press_key((0xFB, 0));
        input.pad.bits = 0x01;
        assert_eq!(
            c.read(&input),
            0x03,
            "left from the keys, right from the pad"
        );
    }

    #[test]
    fn pause_is_the_methods_own_key_or_start() {
        let c = keyboard();
        let mut input = Input::default();
        assert!(!c.pause_pressed(&input));
        input.press_key(c.pause);
        assert!(c.pause_pressed(&input));
        let mut input = Input::default();
        input.pad.start = true;
        assert!(c.pause_pressed(&input));
    }
}
