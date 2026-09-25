//! How much help the player has asked for: the guidance level, training
//! mode, and the record of both for the game in progress.
//!
//! Nothing here reaches the game. The window and the game thread share it:
//! the window changes it from the keyboard and draws it, and the game thread
//! changes it from a gamepad and holds the game while the picker is open.

use super::gamepad::Layout;
use starquake::game::{SeenTeleporter, Training};
use starquake::map::{Openings, Step};
use starquake::pickups::RoomSet;

/// The number of rooms on the planet.
const ROOMS: usize = (starquake::map::COLS * starquake::map::ROWS) as usize;

/// The levels, each including the ones before it (#1), as ZX Sidekick
/// re-cut them after playing (#91). Levels 0 to 3 show only what you could
/// have written down yourself; 4 and up tell you what you could not have
/// known.
pub const LEVELS: [&str; 7] = [
    "Off",
    "Codes and the core",
    "The map you have walked",
    "What you have seen",
    "What you have not",
    "Routes",
    "Everything",
];

/// The CORE OF HEROES table as the panel lists it beside the game's own
/// screen (#90): each entry's initials, the guidance level its game had
/// (`None` for the tape's own entries), and which entry the last game put
/// in.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct Heroes {
    pub names: [[u8; 3]; 8],
    pub levels: [Option<u8>; 8],
    pub this_game: Option<usize>,
}

/// An item lying out on the planet (#93): the room, what it does, whether
/// the core wants it, its graphic from the game, and whether it has been
/// seen lying in a room walked through, which is level 3's half of the
/// map's items; the rest are level 4's.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct Found {
    pub room: u16,
    pub kind: starquake::pickups::Kind,
    pub piece: bool,
    pub graphic: [u8; 32],
    pub seen: bool,
}

/// A security door whose code is shown (#94): its room, the three key code
/// cards it asks for in the game's own graphics, and which of them what is
/// carried answers.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct DoorCode {
    pub room: u16,
    /// The cards by graphic number, and their pictures.
    pub cards: [u8; 3],
    pub graphics: [[u8; 32]; 3],
    pub answered: [bool; 3],
}

/// One of the core's nine slots, for the square at the panel's top left
/// (#91): the piece it takes, in the game's own graphic, whether it is still
/// wanted, and whether that piece is being carried.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct Hole {
    pub graphic: [u8; 32],
    pub open: bool,
    pub carried: bool,
}

/// How much help one game has had: the highest level in use at any point,
/// whether training mode was ever on, and which of its switches (#4). It
/// only ever rises within a game.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub struct Record {
    pub highest: u8,
    pub training: bool,
    pub switches: Training,
}

/// The rows of the picker, top to bottom: the guidance level, training
/// mode's five switches (#4), then two actions.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub enum Setting {
    #[default]
    Level,
    Switch(u8),
    EndGame,
    Exit,
}

/// Where "This will show on your score" stands (#122): asked, or pressed
/// once and waiting for the second press that keeps the change, as the
/// picker's actions wait.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Choice {
    Asked,
    Armed,
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
    training: Training,
    record: Record,
    picker: bool,
    /// The level and training mode when the picker opened, which Undo goes
    /// back to.
    opened: (u8, Training),
    /// "Noted with your score", asked when leaving the picker would add to
    /// the record, and which answer is highlighted.
    asking: Option<Choice>,
    /// The row the picker has highlighted.
    focus: Setting,
    /// An action pressed once, waiting for the second press.
    armed: Option<Setting>,
    /// Whether a game is being played, which is when it can be ended.
    playing: bool,
    /// An action confirmed and not yet carried out.
    requested: Option<Action>,
    /// The teleporters seen this game: their codes for level 1 (#50), and
    /// their rooms for the map.
    teleporters: Vec<SeenTeleporter>,
    /// Every room's openings, for the map (#2). Empty until they are found.
    openings: Vec<Openings>,
    /// The rooms visited in the game being played, or just ended; empty on
    /// the title screen.
    visited: Vec<bool>,
    /// The room BLOB is in, while a game is being played.
    room: Option<u16>,
    /// The rooms holding a core piece still needed, for level 3 (#3), in
    /// the game being played or just ended.
    pieces: RoomSet,
    /// The letters the connected pad carries, for the legends (#88).
    pad: Layout,
    /// Whether the game is held by its pause key, for the notice (#89).
    paused: bool,
    /// The high-score table, while the CORE OF HEROES screen shows it.
    heroes: Option<Heroes>,
    /// The core's nine slots in the game being played or just ended; empty
    /// on the title screen.
    core: Vec<Hole>,
    /// The items lying out on the planet, in the game being played.
    items: Vec<Found>,
    /// The security doors whose codes have been shown, in the order seen.
    doors: Vec<DoorCode>,
    /// Level 5's routes (#52): to the chosen of the nearest missing pieces,
    /// and to the core while a piece it needs is carried; the first door
    /// each has to pass; the piece chosen with Tab, and which of how many
    /// the route leads to; and a Tab pressed and not yet taken.
    route: Option<Vec<Step>>,
    core_route: Option<Vec<Step>>,
    route_doors: [Option<u16>; 2],
    chosen_piece: Option<u16>,
    piece_choice: (u8, u8),
    switch: bool,
    /// Every teleporter and every door on the planet, the seen ones first,
    /// for level 6 (#95).
    every_teleporter: Vec<SeenTeleporter>,
    every_door: Vec<DoorCode>,
    /// Bumped on every change, so a watcher can tell something changed.
    version: u64,
}

impl Guidance {
    /// The guidance level in effect.
    pub fn level(&self) -> u8 {
        self.level
    }

    /// Training mode's switches in effect.
    pub fn training(&self) -> Training {
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

    /// Whether the game is held by its pause key.
    pub fn paused(&self) -> bool {
        self.paused
    }

    /// Notes whether the game is paused, redrawing when that changes.
    pub fn set_paused(&mut self, paused: bool) {
        if self.paused != paused {
            self.paused = paused;
            self.version += 1;
        }
    }

    /// The route to the chosen missing piece, if there is one (#52).
    pub fn route(&self) -> Option<&[Step]> {
        self.route.as_deref()
    }

    /// The route to the core, while a piece it needs is carried.
    pub fn core_route(&self) -> Option<&[Step]> {
        self.core_route.as_deref()
    }

    /// The first room with a security door each route has to pass: the
    /// piece route's, then the core route's.
    pub fn route_doors(&self) -> [Option<u16>; 2] {
        self.route_doors
    }

    pub fn set_routes(
        &mut self,
        route: Option<Vec<Step>>,
        core_route: Option<Vec<Step>>,
        doors: [Option<u16>; 2],
    ) {
        if self.route != route || self.core_route != core_route || self.route_doors != doors {
            self.route = route;
            self.core_route = core_route;
            self.route_doors = doors;
            self.version += 1;
        }
    }

    /// The letters the connected pad carries.
    pub fn pad(&self) -> Layout {
        self.pad
    }

    /// Notes the pad's letters, redrawing the legends when they change.
    pub fn set_pad(&mut self, layout: Layout) {
        if self.pad != layout {
            self.pad = layout;
            self.version += 1;
        }
    }

    /// The piece room chosen with Tab, if any.
    pub fn chosen_piece(&self) -> Option<u16> {
        self.chosen_piece
    }

    /// Which of how many nearest pieces the route leads to, counting from 1.
    pub fn piece_choice(&self) -> (u8, u8) {
        self.piece_choice
    }

    pub fn set_piece_choice(&mut self, chosen: Option<u16>, choice: (u8, u8)) {
        if self.chosen_piece != chosen || self.piece_choice != choice {
            self.chosen_piece = chosen;
            self.piece_choice = choice;
            self.version += 1;
        }
    }

    /// The high-score table, while the CORE OF HEROES screen shows it.
    pub fn heroes(&self) -> Option<Heroes> {
        self.heroes
    }

    pub fn set_heroes(&mut self, heroes: Option<Heroes>) {
        if self.heroes != heroes {
            self.heroes = heroes;
            self.version += 1;
        }
    }

    /// Tab was pressed: the next of the nearest pieces, at level 5 and up.
    pub fn request_switch(&mut self) {
        if self.level >= 5 {
            self.switch = true;
        }
    }

    /// Whether Tab was pressed since this was last asked.
    pub fn take_switch(&mut self) -> bool {
        std::mem::take(&mut self.switch)
    }

    /// The teleporters and doors whose codes the panel shows: those seen,
    /// or at level 6 every one there is (#95).
    pub fn codes(&self) -> (&[SeenTeleporter], &[DoorCode]) {
        if self.level >= 6 {
            (&self.every_teleporter, &self.every_door)
        } else {
            (&self.teleporters, &self.doors)
        }
    }

    pub fn set_every(&mut self, teleporters: Vec<SeenTeleporter>, doors: Vec<DoorCode>) {
        if self.every_teleporter != teleporters || self.every_door != doors {
            self.every_teleporter = teleporters;
            self.every_door = doors;
            self.version += 1;
        }
    }

    pub fn set_doors(&mut self, doors: Vec<DoorCode>) {
        if self.doors != doors {
            self.doors = doors;
            self.version += 1;
        }
    }

    /// The items lying out on the planet, or none before a game.
    pub fn items(&self) -> &[Found] {
        &self.items
    }

    pub fn set_items(&mut self, items: Vec<Found>) {
        if self.items != items {
            self.items = items;
            self.version += 1;
        }
    }

    /// The core's nine slots, or none before a game.
    pub fn core(&self) -> &[Hole] {
        &self.core
    }

    pub fn set_core(&mut self, core: &[Hole]) {
        if self.core != core {
            self.core = core.to_vec();
            self.version += 1;
        }
    }

    pub fn focus(&self) -> Setting {
        self.focus
    }

    pub fn armed(&self) -> Option<Setting> {
        self.armed
    }

    /// The level and training mode as they were when the picker opened,
    /// which Undo goes back to.
    pub fn opened(&self) -> (u8, Training) {
        self.opened
    }

    /// The question, if it is up, and whether it has been pressed once.
    pub fn asking(&self) -> Option<Choice> {
        self.asking
    }

    /// Whether keeping the settings as they are would add to this game's
    /// record: a level above the highest used, or training mode for the
    /// first time. Lowering either never does.
    pub fn raises_record(&self) -> bool {
        self.level > self.record.highest
            || self.record.switches.union(self.training) != self.record.switches
    }

    /// The doors seen this game.
    #[cfg(test)]
    pub fn doors_for_test(&self) -> &[DoorCode] {
        &self.doors
    }

    /// The teleporters seen this game.
    #[cfg(test)]
    pub fn teleporters(&self) -> &[SeenTeleporter] {
        &self.teleporters
    }

    /// Takes the game's list of teleporters seen, if it has changed.
    pub fn set_teleporters(&mut self, seen: &[SeenTeleporter]) {
        if self.teleporters != seen {
            self.teleporters = seen.to_vec();
            self.version += 1;
        }
    }

    /// Every room's openings, by room number; empty until they are found.
    pub fn openings(&self) -> &[Openings] {
        &self.openings
    }

    /// Whether the openings have been found yet.
    pub fn has_openings(&self) -> bool {
        !self.openings.is_empty()
    }

    /// Takes every room's openings, found once.
    pub fn set_openings(&mut self, openings: Vec<Openings>) {
        self.openings = openings;
        self.version += 1;
    }

    /// Whether `room` has been visited.
    pub fn visited(&self, room: u16) -> bool {
        self.visited.get(room as usize).copied().unwrap_or(false)
    }

    /// How many rooms have been visited.
    #[cfg(test)]
    pub fn explored(&self) -> usize {
        self.visited.iter().filter(|&&v| v).count()
    }

    /// The room BLOB is in, while a game is being played.
    pub fn room(&self) -> Option<u16> {
        self.room
    }

    /// Takes the room BLOB is in, or `None` outside a game, if it has
    /// changed. The number the game keeps after its end (512) is no room.
    pub fn set_room(&mut self, room: Option<u16>) {
        let room = room.filter(|&r| (r as usize) < ROOMS);
        if self.room != room {
            self.room = room;
            self.version += 1;
        }
    }

    /// Whether `room` holds a core piece still needed.
    pub fn piece(&self, room: u16) -> bool {
        self.pieces.contains(room)
    }

    /// Takes the rooms holding a core piece still needed, if they have
    /// changed.
    pub fn set_pieces(&mut self, rooms: &RoomSet) {
        if self.pieces != *rooms {
            self.pieces = rooms.clone();
            self.version += 1;
        }
    }

    /// Takes the game's set of rooms not yet visited, if it has changed.
    pub fn set_unvisited(&mut self, unvisited: &RoomSet) {
        let same = self.visited.len() == ROOMS
            && (0..ROOMS).all(|r| self.visited[r] != unvisited.contains(r as u16));
        if !same {
            self.visited = (0..ROOMS).map(|r| !unvisited.contains(r as u16)).collect();
            self.version += 1;
        }
    }

    /// Forgets the game that has ended, its map, pieces and teleporter codes,
    /// once its game-over screens are done: the title screen shows none.
    pub fn forget_game(&mut self) {
        let empty = RoomSet::default();
        if !self.teleporters.is_empty()
            || !self.visited.is_empty()
            || self.room.is_some()
            || self.pieces != empty
            || !self.core.is_empty()
            || !self.items.is_empty()
            || !self.doors.is_empty()
            || !self.every_door.is_empty()
            || self.route.is_some()
            || self.core_route.is_some()
        {
            self.teleporters.clear();
            self.visited.clear();
            self.room = None;
            self.pieces = empty;
            self.core.clear();
            self.items.clear();
            self.doors.clear();
            self.every_teleporter.clear();
            self.every_door.clear();
            self.route = None;
            self.core_route = None;
            self.route_doors = [None; 2];
            self.chosen_piece = None;
            self.piece_choice = (0, 0);
            self.version += 1;
        }
    }

    /// The rows the picker shows: ending a game only while one is played.
    pub fn rows(&self) -> Vec<Setting> {
        let mut rows = vec![Setting::Level];
        rows.extend((0..Training::NAMES.len() as u8).map(Setting::Switch));
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

    /// Opens the picker on its top row.
    pub fn open(&mut self) {
        self.picker = true;
        self.opened = (self.level, self.training);
        self.asking = None;
        self.focus = Setting::Level;
        self.armed = None;
        self.version += 1;
    }

    /// Esc, B or Select. With the question up, the change is undone and
    /// the picker closes (#122). Otherwise leave it.
    pub fn back(&mut self) {
        if self.asking.is_some() {
            (self.level, self.training) = self.opened;
            self.close();
        } else {
            self.leave();
        }
    }

    /// Leaves the picker, asking first if that would add to the record. It
    /// takes two presses to keep, so a reflex press changes nothing.
    fn leave(&mut self) {
        if self.raises_record() {
            self.asking = Some(Choice::Asked);
            self.armed = None;
            self.version += 1;
        } else {
            self.close();
        }
    }

    /// Closes the picker, keeping what was set in it. The record takes the
    /// settings as they are now, so passing through a level on the way to
    /// another does not count as having used it.
    pub fn close(&mut self) {
        self.picker = false;
        self.asking = None;
        self.armed = None;
        self.record.highest = self.record.highest.max(self.level);
        self.record.switches = self.record.switches.union(self.training);
        self.record.training = self.record.switches.any();
        self.version += 1;
    }

    /// Enter or A. With the question up, the first press asks for a second
    /// and the second keeps the change (#122). On a setting it leaves the
    /// picker, as Esc does. On an action the first press asks for a second,
    /// and the second requests the action and closes the picker.
    pub fn enter(&mut self) {
        match self.asking {
            Some(Choice::Asked) => {
                self.asking = Some(Choice::Armed);
                self.version += 1;
                return;
            }
            Some(Choice::Armed) => {
                self.close();
                return;
            }
            None => {}
        }
        let action = match self.focus {
            Setting::Level | Setting::Switch(_) => {
                self.leave();
                return;
            }
            Setting::EndGame => Action::EndGame,
            Setting::Exit => Action::Exit,
        };
        if self.armed == Some(self.focus) {
            self.requested = Some(action);
            // An action is not a decision about the settings: anything that
            // would add to the record without being confirmed is undone, so
            // an ended game's score note cannot pick it up by accident.
            if self.raises_record() {
                (self.level, self.training) = self.opened;
            }
            self.close();
        } else {
            self.armed = Some(self.focus);
            self.version += 1;
        }
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
        if self.asking.is_some() {
            self.disarm();
            return;
        }
        let rows = self.rows();
        let at = rows.iter().position(|&r| r == self.focus).unwrap_or(0) as isize;
        let to = (at + by).clamp(0, rows.len() as isize - 1) as usize;
        self.focus = rows[to];
        self.armed = None;
        self.version += 1;
    }

    /// With the question pressed once, anything but a second Enter or A
    /// takes it back to asking (#122).
    fn disarm(&mut self) {
        if self.asking == Some(Choice::Armed) {
            self.asking = Some(Choice::Asked);
            self.version += 1;
        }
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
    /// step, in effect at once. It is recorded when the picker closes.
    pub fn change(&mut self, up: bool) {
        if self.asking.is_some() {
            self.disarm();
            return;
        }
        let max = LEVELS.len() as u8 - 1;
        match (self.focus, up) {
            (Setting::Level, true) => self.level = (self.level + 1).min(max),
            (Setting::Level, false) => self.level = self.level.saturating_sub(1),
            (Setting::Switch(i), on) => self.training.0[usize::from(i)] = on,
            (Setting::EndGame | Setting::Exit, _) => return,
        }
        self.version += 1;
    }

    /// Puts a level into effect and records it, outside the picker. For
    /// tests, which start from a setting without going through the picker.
    #[cfg(test)]
    pub fn set_level(&mut self, level: u8) {
        self.level = level.min(LEVELS.len() as u8 - 1);
        self.record.highest = self.record.highest.max(self.level);
        self.version += 1;
    }

    /// Puts training mode into effect or out of it and records it. For
    /// tests, which start from a setting without going through the picker.
    #[cfg(test)]
    pub fn set_training(&mut self, on: bool) {
        self.training = Training([on; 5]);
        self.record.switches = self.record.switches.union(self.training);
        self.record.training = self.record.switches.any();
        self.version += 1;
    }

    /// A new game has started: its record begins with what is in use now.
    pub fn new_game(&mut self) {
        self.record = Record {
            highest: self.level,
            training: self.training.any(),
            switches: self.training,
        };
        self.version += 1;
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    /// Moves the picker's focus down to `row`.
    fn down_to(g: &mut Guidance, row: Setting) {
        while g.focus() != row {
            g.focus_down();
        }
    }

    #[test]
    fn starts_with_no_help() {
        let g = Guidance::default();
        assert_eq!(g.level(), 0);
        assert!(!g.training().any());
        assert_eq!(g.record(), Record::default());
        assert!(!g.picker_open());
    }

    #[test]
    fn the_record_only_rises() {
        let mut g = Guidance::default();
        g.set_level(3);
        g.set_level(1);
        assert_eq!(g.record().highest, 3);
        g.set_training(true);
        g.set_training(false);
        assert!(g.record().training);
    }

    #[test]
    fn changes_are_in_effect_at_once_and_kept_on_closing() {
        let mut g = Guidance::default();
        g.open();
        g.change(true);
        g.change(true);
        assert_eq!(g.level(), 2, "in effect at once");
        g.focus_down();
        g.change(true);
        assert!(g.training().0[0], "the first switch, full energy");
        g.close();
        assert_eq!(g.level(), 2, "kept");
        assert!(g.training().0[0], "kept");
    }

    #[test]
    fn only_what_is_in_effect_on_closing_is_recorded() {
        let mut g = Guidance::default();
        g.open();
        for _ in 0..5 {
            g.change(true);
        }
        assert_eq!(g.record().highest, 0, "not while the picker is open");
        for _ in 0..4 {
            g.change(false);
        }
        g.focus_down();
        g.change(true);
        g.change(false);
        g.close();
        assert_eq!(
            g.record(),
            Record {
                highest: 1,
                training: false,
                switches: Training::default(),
            }
        );
    }

    #[test]
    fn lowering_or_browsing_never_asks() {
        let mut g = Guidance::default();
        g.set_level(3);
        g.open();
        g.change(false);
        g.back();
        assert!(!g.picker_open(), "a lower level: no question");
        assert_eq!(g.level(), 2);

        g.open();
        g.change(true);
        g.change(true);
        g.change(false);
        g.back();
        assert!(!g.picker_open(), "back to level 3, already recorded");
    }

    #[test]
    fn raising_asks_and_one_press_keeps_nothing_yet() {
        let mut g = Guidance::default();
        g.open();
        g.change(true);
        g.change(true);
        g.back();
        assert!(g.picker_open());
        assert_eq!(g.asking(), Some(Choice::Asked));
        g.enter();
        assert!(g.picker_open(), "one press only asks again");
        assert_eq!(g.asking(), Some(Choice::Armed));
        assert_eq!(g.record(), Record::default());
    }

    #[test]
    fn a_second_press_keeps_and_records() {
        let mut g = Guidance::default();
        g.open();
        g.focus_down();
        g.change(true);
        g.enter();
        assert_eq!(
            g.asking(),
            Some(Choice::Asked),
            "Enter on a setting asks too"
        );
        g.enter();
        g.enter();
        assert!(!g.picker_open());
        assert!(g.training().any());
        assert!(g.record().training);
    }

    #[test]
    fn back_from_the_question_undoes_and_closes() {
        let mut g = Guidance::default();
        g.open();
        g.change(true);
        g.back();
        g.enter();
        g.back();
        assert!(!g.picker_open());
        assert_eq!(g.asking(), None);
        assert_eq!(g.level(), 0, "undone");
        assert_eq!(g.record(), Record::default());
    }

    #[test]
    fn anything_else_takes_a_first_press_back() {
        let mut g = Guidance::default();
        g.open();
        g.change(true);
        g.back();
        for step in [
            |g: &mut Guidance| g.change(true),
            |g: &mut Guidance| g.change(false),
            |g: &mut Guidance| g.focus_up(),
            |g: &mut Guidance| g.focus_down(),
        ] {
            g.enter();
            assert_eq!(g.asking(), Some(Choice::Armed));
            step(&mut g);
            assert_eq!(g.asking(), Some(Choice::Asked), "disarmed");
            assert_eq!(g.level(), 1, "and nothing else changed");
        }
    }

    #[test]
    fn ending_a_game_drops_unconfirmed_raises() {
        let mut g = Guidance::default();
        g.set_playing(true);
        g.open();
        g.change(true);
        down_to(&mut g, Setting::EndGame);
        g.enter();
        g.enter();
        assert!(g.take(Action::EndGame));
        assert_eq!(g.level(), 0);
        assert_eq!(g.record(), Record::default());
    }

    #[test]
    fn levels_stop_at_the_ends() {
        let mut g = Guidance::default();
        g.open();
        g.change(false);
        assert_eq!(g.level(), 0);
        for _ in 0..10 {
            g.change(true);
        }
        assert_eq!(g.level(), 6, "the re-cut's top level, Everything");
    }

    #[test]
    fn an_action_needs_two_presses() {
        let mut g = Guidance::default();
        g.set_playing(true);
        g.open();
        down_to(&mut g, Setting::EndGame);
        assert_eq!(g.focus(), Setting::EndGame);
        g.enter();
        assert_eq!(g.armed(), Some(Setting::EndGame));
        assert!(!g.take(Action::EndGame), "one press does nothing yet");
        g.enter();
        assert!(g.take(Action::EndGame));
        assert!(!g.picker_open(), "the picker closes");
    }

    #[test]
    fn moving_away_cancels_a_first_press() {
        let mut g = Guidance::default();
        g.open();
        for _ in 0..10 {
            g.focus_down();
        }
        assert_eq!(g.focus(), Setting::Exit);
        g.enter();
        g.focus_up();
        g.focus_down();
        g.enter();
        assert!(!g.take(Action::Exit), "the first press was cancelled");
    }

    #[test]
    fn the_picker_opens_on_its_top_row() {
        let mut g = Guidance::default();
        g.set_playing(true);
        g.open();
        for _ in 0..5 {
            g.focus_down();
        }
        g.close();
        g.open();
        assert_eq!(g.focus(), Setting::Level);
    }

    #[test]
    fn ending_a_game_is_offered_only_while_playing() {
        let mut g = Guidance::default();
        let switches = (0..5).map(Setting::Switch);
        let rows: Vec<Setting> = std::iter::once(Setting::Level)
            .chain(switches.clone())
            .chain([Setting::Exit])
            .collect();
        assert_eq!(g.rows(), rows);
        g.set_playing(true);
        let rows: Vec<Setting> = std::iter::once(Setting::Level)
            .chain(switches)
            .chain([Setting::EndGame, Setting::Exit])
            .collect();
        assert_eq!(g.rows(), rows);
    }

    #[test]
    fn each_switch_is_its_own_row_and_the_record_names_them() {
        let mut g = Guidance::default();
        g.open();
        for _ in 0..4 {
            g.focus_down();
        }
        assert_eq!(g.focus(), Setting::Switch(3), "endless lives");
        g.change(true);
        assert!(g.raises_record(), "a switch not used yet this game");
        g.close();
        assert_eq!(g.training(), Training([false, false, false, true, false]));
        assert!(g.record().training);
        assert_eq!(g.record().switches, g.training());
    }

    #[test]
    fn teleporters_are_taken_only_when_they_change() {
        let mut g = Guidance::default();
        let before = g.version();
        g.set_teleporters(&[]);
        assert_eq!(g.version(), before, "nothing new, nothing to redraw");
        let seen = SeenTeleporter {
            room: 40,
            code: *b"ABCDE",
        };
        g.set_teleporters(&[seen]);
        assert_eq!(g.teleporters(), [seen]);
        assert!(g.version() > before);
    }

    #[test]
    fn the_map_is_taken_only_when_it_changes() {
        let mut g = Guidance::default();
        assert_eq!(g.explored(), 0, "nothing explored before a game");
        let mut unvisited = RoomSet([0xFF; 64]);
        unvisited.set(97, false);
        unvisited.set(98, false);
        g.set_unvisited(&unvisited);
        g.set_room(Some(98));
        assert_eq!(g.explored(), 2);
        assert!(g.visited(97) && !g.visited(96));
        assert_eq!(g.room(), Some(98));

        let before = g.version();
        g.set_unvisited(&unvisited);
        g.set_room(Some(98));
        assert_eq!(g.version(), before, "nothing new, nothing to redraw");

        let mut pieces = RoomSet::default();
        pieces.set(300, true);
        g.set_pieces(&pieces);
        assert!(g.piece(300) && !g.piece(98));
        let after = g.version();
        assert!(after > before);
        g.set_pieces(&pieces);
        assert_eq!(g.version(), after, "the same pieces, nothing to redraw");

        g.set_room(Some(512));
        assert_eq!(
            g.room(),
            None,
            "512 is where the game leaves it, not a room"
        );
        assert!(g.version() > before);

        g.set_teleporters(&[SeenTeleporter {
            room: 40,
            code: *b"ABCDE",
        }]);
        g.forget_game();
        assert_eq!(g.explored(), 0, "the title screen shows no map");
        assert!(g.teleporters().is_empty(), "nor any codes");
        assert!(!g.piece(300), "nor any pieces");
        assert_eq!(g.room(), None);
        let forgotten = g.version();
        g.forget_game();
        assert_eq!(g.version(), forgotten, "nothing left to forget");
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
                training: false,
                switches: Training::default(),
            }
        );
    }
}
