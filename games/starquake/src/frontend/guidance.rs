//! How much help the player has asked for: the guidance level, training
//! mode, and the record of both for the game in progress.
//!
//! Nothing here reaches the game. The window and the game thread share it:
//! the window changes it from the keyboard and draws it, and the game thread
//! changes it from a gamepad and holds the game while the picker is open.

/// The levels, each including the ones before it (#1).
pub const LEVELS: [&str; 6] = [
    "Off",
    "Teleporter codes",
    "Map",
    "Missing pieces",
    "Arrow, known routes",
    "Arrow, whole map",
];

/// How much help one game has had: the highest level in use at any point,
/// and whether training mode was ever on. It only ever rises within a game.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub struct Record {
    pub highest: u8,
    pub training: bool,
}

/// The rows of the picker, top to bottom: two settings, then two actions.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub enum Setting {
    #[default]
    Level,
    Training,
    EndGame,
    Exit,
}

/// What the picker was asked to do, once confirmed.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Action {
    /// Abandon the game in progress, as A S D F G does.
    EndGame,
    /// Close the program.
    Exit,
}

#[derive(Clone, Debug, Default)]
pub struct Guidance {
    level: u8,
    training: bool,
    record: Record,
    picker: bool,
    /// The row the picker has highlighted.
    focus: Setting,
    /// An action pressed once, waiting for the second press.
    armed: Option<Setting>,
    /// Whether a game is being played, which is when it can be ended.
    playing: bool,
    /// An action confirmed and not yet carried out.
    requested: Option<Action>,
    /// Bumped on every change, so a watcher can tell something changed.
    version: u64,
}

impl Guidance {
    pub fn level(&self) -> u8 {
        self.level
    }

    pub fn training(&self) -> bool {
        self.training
    }

    pub fn record(&self) -> Record {
        self.record
    }

    pub fn picker_open(&self) -> bool {
        self.picker
    }

    pub fn version(&self) -> u64 {
        self.version
    }

    pub fn focus(&self) -> Setting {
        self.focus
    }

    pub fn armed(&self) -> Option<Setting> {
        self.armed
    }

    /// The rows the picker shows: ending a game only while one is played.
    pub fn rows(&self) -> Vec<Setting> {
        let mut rows = vec![Setting::Level, Setting::Training];
        if self.playing {
            rows.push(Setting::EndGame);
        }
        rows.push(Setting::Exit);
        rows
    }

    /// Whether a game is being played, as the game thread sees it.
    pub fn set_playing(&mut self, playing: bool) {
        self.playing = playing;
        if !playing && self.focus == Setting::EndGame {
            self.focus = Setting::Exit;
        }
        if self.armed == Some(Setting::EndGame) {
            self.armed = None;
        }
        self.version += 1;
    }

    /// Up and down in the picker: which row is highlighted. Moving away
    /// from an action that was pressed once cancels it.
    pub fn focus_up(&mut self) {
        self.move_focus(-1);
    }

    pub fn focus_down(&mut self) {
        self.move_focus(1);
    }

    fn move_focus(&mut self, by: isize) {
        let rows = self.rows();
        let at = rows.iter().position(|&r| r == self.focus).unwrap_or(0) as isize;
        let to = (at + by).clamp(0, rows.len() as isize - 1) as usize;
        self.focus = rows[to];
        self.armed = None;
        self.version += 1;
    }

    /// Enter on the highlighted row: an action needs pressing twice, and on
    /// the second press it is requested and the picker closes.
    pub fn activate(&mut self) {
        let action = match self.focus {
            Setting::EndGame => Action::EndGame,
            Setting::Exit => Action::Exit,
            Setting::Level | Setting::Training => return,
        };
        if self.armed == Some(self.focus) {
            self.armed = None;
            self.requested = Some(action);
            self.picker = false;
        } else {
            self.armed = Some(self.focus);
        }
        self.version += 1;
    }

    /// Takes the confirmed action, if there is one and it is `which`.
    pub fn take(&mut self, which: Action) -> bool {
        if self.requested == Some(which) {
            self.requested = None;
            true
        } else {
            false
        }
    }

    /// Left and right in the picker: the highlighted setting down or up a
    /// step, taking effect at once.
    pub fn change(&mut self, up: bool) {
        match (self.focus, up) {
            (Setting::Level, true) => self.level_up(),
            (Setting::Level, false) => self.level_down(),
            (Setting::Training, on) => self.set_training(on),
            (Setting::EndGame | Setting::Exit, _) => {}
        }
    }

    pub fn set_level(&mut self, level: u8) {
        self.level = level.min(LEVELS.len() as u8 - 1);
        self.record.highest = self.record.highest.max(self.level);
        self.version += 1;
    }

    pub fn level_up(&mut self) {
        self.set_level(self.level.saturating_add(1));
    }

    pub fn level_down(&mut self) {
        self.set_level(self.level.saturating_sub(1));
    }

    pub fn set_training(&mut self, on: bool) {
        self.training = on;
        self.record.training |= on;
        self.version += 1;
    }

    pub fn toggle_picker(&mut self) {
        self.picker = !self.picker;
        self.armed = None;
        self.version += 1;
    }

    /// A new game has started: its record begins with what is in use now.
    pub fn new_game(&mut self) {
        self.record = Record {
            highest: self.level,
            training: self.training,
        };
        self.version += 1;
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn starts_with_no_help() {
        let g = Guidance::default();
        assert_eq!(g.level(), 0);
        assert!(!g.training());
        assert_eq!(g.record(), Record::default());
        assert!(!g.picker_open());
    }

    #[test]
    fn the_record_only_rises() {
        let mut g = Guidance::default();
        g.set_level(3);
        g.level_down();
        g.level_down();
        assert_eq!(g.level(), 1);
        assert_eq!(g.record().highest, 3);
        g.set_training(true);
        g.set_training(false);
        assert!(!g.training());
        assert!(g.record().training);
    }

    #[test]
    fn levels_stop_at_the_ends() {
        let mut g = Guidance::default();
        g.level_down();
        assert_eq!(g.level(), 0);
        for _ in 0..10 {
            g.level_up();
        }
        assert_eq!(g.level(), 5);
    }

    #[test]
    fn left_and_right_change_the_highlighted_setting() {
        let mut g = Guidance::default();
        assert_eq!(g.focus(), Setting::Level);
        g.change(true);
        g.change(true);
        assert_eq!(g.level(), 2);
        g.focus_down();
        g.change(true);
        assert!(g.training());
        assert_eq!(g.level(), 2, "the level stays put");
        g.change(true);
        assert!(g.training(), "on stays on");
        g.change(false);
        assert!(!g.training());
        g.focus_up();
        g.change(false);
        assert_eq!(g.level(), 1);
    }

    #[test]
    fn an_action_needs_two_presses() {
        let mut g = Guidance::default();
        g.set_playing(true);
        g.toggle_picker();
        g.focus_down();
        g.focus_down();
        assert_eq!(g.focus(), Setting::EndGame);
        g.activate();
        assert_eq!(g.armed(), Some(Setting::EndGame));
        assert!(!g.take(Action::EndGame), "one press does nothing yet");
        g.activate();
        assert!(g.take(Action::EndGame));
        assert!(!g.picker_open(), "the picker closes");
    }

    #[test]
    fn moving_away_cancels_a_first_press() {
        let mut g = Guidance::default();
        g.toggle_picker();
        for _ in 0..5 {
            g.focus_down();
        }
        assert_eq!(g.focus(), Setting::Exit);
        g.activate();
        g.focus_up();
        g.focus_down();
        g.activate();
        assert!(!g.take(Action::Exit), "the first press was cancelled");
    }

    #[test]
    fn ending_a_game_is_offered_only_while_playing() {
        let mut g = Guidance::default();
        assert_eq!(g.rows(), [Setting::Level, Setting::Training, Setting::Exit]);
        g.set_playing(true);
        assert_eq!(
            g.rows(),
            [
                Setting::Level,
                Setting::Training,
                Setting::EndGame,
                Setting::Exit
            ]
        );
    }

    #[test]
    fn a_new_game_starts_its_record_from_what_is_in_use() {
        let mut g = Guidance::default();
        g.set_level(4);
        g.set_training(true);
        g.set_level(2);
        g.set_training(false);
        g.new_game();
        assert_eq!(g.level(), 2, "the chosen level is kept");
        assert_eq!(
            g.record(),
            Record {
                highest: 2,
                training: false
            }
        );
    }
}
