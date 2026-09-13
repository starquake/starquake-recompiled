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

/// The two settings in the picker, top to bottom.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub enum Setting {
    #[default]
    Level,
    Training,
}

#[derive(Clone, Debug, Default)]
pub struct Guidance {
    level: u8,
    training: bool,
    record: Record,
    picker: bool,
    /// The setting the picker has highlighted.
    focus: Setting,
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

    /// Up and down in the picker: which setting is highlighted.
    pub fn focus_up(&mut self) {
        self.focus = Setting::Level;
        self.version += 1;
    }

    pub fn focus_down(&mut self) {
        self.focus = Setting::Training;
        self.version += 1;
    }

    /// Left and right in the picker: the highlighted setting down or up a
    /// step, taking effect at once.
    pub fn change(&mut self, up: bool) {
        match (self.focus, up) {
            (Setting::Level, true) => self.level_up(),
            (Setting::Level, false) => self.level_down(),
            (Setting::Training, on) => self.set_training(on),
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
