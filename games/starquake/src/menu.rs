//! The title screen and the menu around a game: choosing how to play,
//! defining your own keys, and quitting.
//!
//! Neither menu loop here has a wait in it on the original: each goes round
//! as fast as it can redraw itself, reading the keyboard every time. On a
//! real Spectrum the title menu manages about 13 turns a second and the
//! define-keys loop about 344, so one is slower than the frames the [`Host`]
//! gives us and the other much faster. [`crate::host::Pacer`] spreads both
//! over frames at their own rate.

use crate::controls::key_code;
use crate::game::Game;
use crate::host::Host;

/// Texts and tables in the original.
mod at {
    /// The title line: letters alternating with graphics from a set of UDGs
    /// used only here.
    pub const TITLE: usize = 0x5EAD;
    /// The five control-method options.
    pub const OPTIONS: [usize; 5] = [0x5EE3, 0x5EFC, 0x5F11, 0x5F2C, 0x5F3E];
    /// Everything printed below them.
    pub const REST: usize = 0x5F51;
    pub const JOYSTICK: usize = 0x5F89;
    pub const KEYBOARD: usize = 0x5F97;
    /// Key names of control method 4. (The player-defined ones are kept in
    /// the game state, since they change.)
    pub const METHOD4_KEYS: usize = 0x5E66;
    pub const QUIT: usize = 0x6068;
    pub const GOODBYE: usize = 0x60AD;
    /// "HIT KEY REQUIRED", which ends with the first prompt.
    pub const DEFINE: usize = 0x61AF;
    /// The prompts after it: right, down, up, fire, pause.
    pub const PROMPTS: [usize; 5] = [0x61D6, 0x61E6, 0x61F6, 0x6206, 0x6216];
    /// The keyboard as the define-keys screen lays it out: 4 rows of 10.
    pub const KEY_LAYOUT: usize = 0x60E1;
}

/// Tiles the title screen and the menu draw.
const TITLE_LEFT: u8 = 0x88;
const TITLE_RIGHT: u8 = 0x89;
const STAR: u8 = 0x90;
const GOODBYE_TILE: u8 = 0x56;

/// Turns a real Spectrum manages in a second, for each of the two loops that
/// pace themselves by how fast they redraw. Both are measured from the
/// original in the interpreter, and `sq-verify` checks the first against it.
pub const TURNS_PER_SECOND: u32 = 12;
pub const DEFINE_TURNS_PER_SECOND: u32 = 344;

/// How many keys the define-keys screen shows, and how many are defined.
const KEYS: usize = 40;
const DEFINED: usize = 6;

/// What the player chose to do.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Start {
    /// Play, with this control method.
    Play(u8),
    Quit,
}

impl Game {
    /// Prints the key names the menu lists beside a control method.
    fn print_key_names(&mut self, names: [u8; 5]) {
        self.print_bytes(&names);
    }

    /// Sets the ink for the next menu option: the option matching the
    /// control method in use is drawn in the flashing colour.
    fn option_ink(&mut self, option: u8, ink: u8) {
        let colour = if self.control_method == option {
            ink
        } else {
            3
        };
        self.print_bytes(&[0x10, colour]);
    }

    /// Draws the list of options, with the one in use highlighted.
    fn draw_options(&mut self, ink: u8) {
        for (i, &text) in at::OPTIONS.iter().enumerate() {
            self.option_ink(i as u8 + 1, ink);
            self.print_text(text);
            match i {
                // The three joystick options.
                0..=2 => self.print_text(at::JOYSTICK),
                3 => {
                    self.print_text(at::KEYBOARD);
                    let names: [u8; 5] = self.assets.ram[at::METHOD4_KEYS..at::METHOD4_KEYS + 5]
                        .try_into()
                        .expect("five key names");
                    self.print_key_names(names);
                }
                _ => {
                    // Whatever the player last defined, not what the snapshot
                    // happened to have saved.
                    let names = self.udk;
                    self.print_text(at::KEYBOARD);
                    self.print_key_names(names);
                }
            }
        }
        self.print_text(at::REST);
    }

    /// Draws the title screen: the logo and the list of options.
    pub fn title_draw(&mut self) {
        self.message_screen(5);
        self.draw_tile(TITLE_LEFT, 0x16, 0x09);
        self.draw_tile(TITLE_RIGHT, 0x16, 0x11);
        // The title line has UDGs of its own.
        self.title_udg = true;
        self.print_text(at::TITLE);
        self.title_udg = false;
        self.draw_options(7);
    }

    /// The title screen, with a tune that plays until a key is pressed.
    pub fn title_screen(&mut self, host: &mut dyn Host) {
        self.title_draw();
        self.tune_until_key(host, 3);
    }

    /// The title screen and the menu under it. Returns what to do next.
    pub fn menu(&mut self, host: &mut dyn Host) -> Start {
        let start = self.menu_choice(host);
        self.on_title = false;
        start
    }

    fn menu_choice(&mut self, host: &mut dyn Host) -> Start {
        loop {
            self.on_title = true;
            self.title_screen(host);
            // The highlight flashes between these two colours.
            let mut ink = 7u8;
            let mut countdown = 2u8;
            let mut pacer = crate::host::Pacer::new(TURNS_PER_SECOND);
            'menu: loop {
                self.sync(host);
                for _ in 0..pacer.turns() {
                    self.draw_options(ink);
                    countdown -= 1;
                    if countdown == 0 {
                        countdown = 2;
                        ink = 9 - ink;
                        if ink == 7 {
                            // A star twinkles in the top right, and now and then
                            // the corners are redrawn in a new colour.
                            self.restore_ptr &= 0x00FF;
                            self.draw_tile(STAR, 0, 0x1E);
                            self.rng.step();
                            if self.rng.lo() < 0x28 {
                                self.draw_corners();
                            }
                        }
                    }
                    let key = key_code(&self.assets.ram, &self.input);
                    match key {
                        b'Q' => {
                            self.on_title = false;
                            if self.quit_confirmed(host) {
                                return Start::Quit;
                            }
                            break 'menu;
                        }
                        b'6' => {
                            self.on_title = false;
                            self.define_keys(host);
                            self.control_method = 5;
                            break 'menu;
                        }
                        b'0' => return Start::Play(self.control_method),
                        b'1'..=b'5' => {
                            // The original refuses Kempston when it cannot find
                            // the interface; here the arrow keys always are one.
                            let method = key - b'0';
                            if self.control_method != method {
                                self.control_method = method;
                                self.request_effect(0x0C);
                            }
                        }
                        _ => {}
                    }
                }
            }
        }
    }

    /// Draws the quit confirmation.
    pub fn quit_screen(&mut self) {
        self.message_screen(4);
        self.print_text(at::QUIT);
    }

    /// "ARE YOU SURE...": true if the player said yes.
    fn quit_confirmed(&mut self, host: &mut dyn Host) -> bool {
        self.quit_screen();
        if self.ask_key(host, |k| k != 0) != b'Y' {
            return false;
        }
        self.print_text(at::GOODBYE);
        self.draw_tile(GOODBYE_TILE, 0x0C, 0x0C);
        // The original then wipes its map and resets the Spectrum.
        self.pause_frames(host, 255);
        true
    }

    /// Draws the keyboard, with the keys already chosen shown as markers.
    fn draw_key_table(&mut self, table: &[u8; KEYS]) {
        for (k, &name) in table.iter().enumerate() {
            let (row, i) = (k / 10, k % 10);
            // The keyboard's rows are staggered, and the last one is not.
            let col = if row == 3 { 0 } else { row as u8 } + i as u8 * 3;
            let (paper, bright) = if name >= 0x90 { (2, 1) } else { (1, 0) };
            let r = row as u8 * 3;
            self.print_bytes(&[
                0x10,
                7,
                0x11,
                paper,
                0x16,
                r,
                col,
                0x13,
                bright,
                name,
                0x10,
                0,
                b'+',
                0x16,
                r + 1,
                col,
                0x8C,
                b',',
                0x10,
                5,
                0x13,
                1,
                0x11,
                0,
            ]);
        }
    }

    /// Waits for one key to be chosen, echoes it, and marks it used.
    fn define_key(
        &mut self,
        host: &mut dyn Host,
        table: &mut [u8; KEYS],
        index: u8,
        flash: &mut (u8, u8),
    ) -> u8 {
        self.wait_keys_released(host);
        let mut pacer = crate::host::Pacer::new(DEFINE_TURNS_PER_SECOND);
        loop {
            self.sync(host);
            // This loop runs far faster than the frame rate on a Spectrum, so
            // a frame is worth about seven turns of it. One turn per frame
            // flashed the dash once a second instead of about seven times.
            for _ in 0..pacer.turns() {
                let (countdown, ink) = &mut *flash;
                *countdown -= 1;
                if *countdown == 0 {
                    *countdown = 0x32;
                    *ink = 9 - *ink;
                }
                // A dash flashes where the key will appear.
                let ink = *ink;
                self.print_bytes(&[0x10, ink, b'-', 0x08, 0x10, 7]);
            }
            let key = key_code(&self.assets.ram, &self.input);
            if key == 0 {
                continue;
            }
            // The shifts, enter and space are named by their own graphics.
            let name = if key >= 0x21 {
                key
            } else {
                let n = key.wrapping_add(0x5A);
                if n == 0x7A { 0x2A } else { n }
            };
            let Some(slot) = table.iter().position(|&n| n == name) else {
                continue;
            };
            self.print_bytes(&[name]);
            table[slot] = 0x90 + index;
            self.request_effect(1);
            self.draw_key_table(table);
            return name;
        }
    }

    /// The define-keys screen as it first appears, with every key still
    /// free. Returns the table of key names it drew.
    ///
    /// # Panics
    ///
    /// If the loaded game data is too short to hold what the original keeps
    /// there, which means the file was not Starquake.
    pub fn define_keys_draw(&mut self) -> [u8; KEYS] {
        self.clear_screen();
        let table: [u8; KEYS] = self.assets.ram[at::KEY_LAYOUT..at::KEY_LAYOUT + KEYS]
            .try_into()
            .expect("40 key names");
        self.draw_key_table(&table);
        table
    }

    /// Define your own keys: six keys picked off a drawing of the keyboard.
    pub fn define_keys(&mut self, host: &mut dyn Host) {
        let mut table = self.define_keys_draw();
        self.print_text(at::DEFINE);
        let mut flash = (0x2Au8, 2u8);
        for i in 0..DEFINED {
            if i > 0 {
                self.print_text(at::PROMPTS[i - 1]);
            }
            let name = self.define_key(host, &mut table, i as u8, &mut flash);
            match i {
                0..=4 => self.udk[i] = name,
                _ => self.udk_pause = name,
            }
        }
        self.pause_frames(host, 120);
    }
}
