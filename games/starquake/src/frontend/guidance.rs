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

#[derive(Clone, Debug, Default)]
pub struct Guidance {
    level: u8,
    training: bool,
    record: Record,
    picker: bool,
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

    pub fn toggle_training(&mut self) {
        self.training = !self.training;
        self.record.training |= self.training;
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
        g.toggle_training();
        g.toggle_training();
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
    fn a_new_game_starts_its_record_from_what_is_in_use() {
        let mut g = Guidance::default();
        g.set_level(4);
        g.toggle_training();
        g.set_level(2);
        g.toggle_training();
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
