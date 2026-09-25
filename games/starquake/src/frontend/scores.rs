//! Keeping the high-score table between runs (#90), with the guidance each
//! entry was played with. The table is the game's own; the game hands it to
//! the host when the CORE OF HEROES screen is about to show it
//! ([`starquake::host::Host::heroes`]), and this keeps a copy in
//! `high-scores.txt` beside the kept tape, which is put back into the game
//! when the program starts.

use std::path::{Path, PathBuf};

use super::guidance::{LEVELS, Record};

const FILE: &str = "high-scores.txt";

/// Entries in the table, and bytes an entry: three initials, six score
/// digits and the percentage, as the game keeps them.
pub const HEROES: usize = 8;
const ENTRY: usize = 10;

/// The table and, for each entry, the highest guidance level its game had:
/// `None` for an entry the tape came with.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct Kept {
    pub table: Vec<u8>,
    pub levels: [Option<u8>; HEROES],
}

impl Kept {
    /// The table as the tape has it, none of it played here.
    pub fn from_tape(table: &[u8]) -> Kept {
        Kept {
            table: table.to_vec(),
            levels: [None; HEROES],
        }
    }

    /// An entry's three initials.
    pub fn name(&self, i: usize) -> [u8; 3] {
        self.table[i * ENTRY..i * ENTRY + 3].try_into().unwrap()
    }

    /// The file's text: a line an entry, best first, of its initials, score,
    /// percentage and guidance level, or `-` for the tape's own.
    pub fn text(&self) -> String {
        (0..HEROES)
            .map(|i| {
                let e = &self.table[i * ENTRY..(i + 1) * ENTRY];
                format!(
                    "{} {} {} {}\n",
                    String::from_utf8_lossy(&e[..3]),
                    String::from_utf8_lossy(&e[3..9]),
                    e[9],
                    self.levels[i].map_or("-".to_string(), |l| l.to_string())
                )
            })
            .collect()
    }

    /// A table from the file's text; `None` when it is not one.
    pub fn parse(text: &str) -> Option<Kept> {
        let lines: Vec<&str> = text.lines().filter(|l| !l.trim().is_empty()).collect();
        if lines.len() != HEROES {
            return None;
        }
        let mut kept = Kept::from_tape(&[0; HEROES * ENTRY]);
        for (i, line) in lines.iter().enumerate() {
            let name = line.as_bytes().get(..3)?;
            if !name.iter().all(|b| (0x20..0x7F).contains(b)) {
                return None;
            }
            let mut rest = line.get(3..)?.split_whitespace();
            let score = rest.next()?.as_bytes();
            if score.len() != 6 || !score.iter().all(u8::is_ascii_digit) {
                return None;
            }
            let percent: u8 = rest.next()?.parse().ok()?;
            let level = match rest.next()? {
                "-" => None,
                // The file is written by this program, which has as many
                // levels as `LEVELS`.
                l => Some(
                    l.parse::<u8>()
                        .ok()
                        .filter(|&l| usize::from(l) < LEVELS.len())?,
                ),
            };
            if rest.next().is_some() {
                return None;
            }
            let e = &mut kept.table[i * ENTRY..(i + 1) * ENTRY];
            e[..3].copy_from_slice(name);
            e[3..9].copy_from_slice(score);
            e[9] = percent;
            kept.levels[i] = level;
        }
        Some(kept)
    }
}

/// What happens to the table around a game over.
#[derive(Debug)]
pub struct Keeper {
    pub kept: Kept,
    /// Which entry the last game put in, for the panel to mark.
    pub this_game: Option<usize>,
    /// Whether the file may be written: not when one is there that could
    /// not be read, which is left alone.
    writable: bool,
    /// The table to put back once the screen is done, after a game with
    /// training mode.
    restore: Option<Vec<u8>>,
}

impl Keeper {
    /// Starts from the file's text, if there is a file, or from the tape's
    /// own table `shipped`.
    pub fn new(file: Option<&str>, shipped: &[u8]) -> Keeper {
        let read = file.map(Kept::parse);
        Keeper {
            kept: read
                .clone()
                .flatten()
                .unwrap_or_else(|| Kept::from_tape(shipped)),
            this_game: None,
            writable: !matches!(read, Some(None)),
            restore: None,
        }
    }

    /// At the CORE OF HEROES screen, showing `table` after a game with
    /// `record` that put in entry `new`: the table to save when it changed,
    /// `None` otherwise. A game with training mode is not kept: its table is
    /// shown, then put back ([`Keeper::shown`]).
    pub fn heroes(&mut self, table: &[u8], new: Option<usize>, record: Record) -> Option<&Kept> {
        if record.training {
            self.this_game = None;
            if new.is_some() {
                self.restore = Some(self.kept.table.clone());
            }
            return None;
        }
        self.this_game = new;
        let i = new?;
        self.kept.levels.copy_within(i..HEROES - 1, i + 1);
        self.kept.levels[i] = Some(record.highest);
        self.kept.table = table.to_vec();
        self.writable.then_some(&self.kept)
    }

    /// The table has been shown: the one to put back, if the game was not
    /// to be kept.
    pub fn shown(&mut self) -> Option<Vec<u8>> {
        self.restore.take()
    }
}

/// Where the table is kept: beside the kept tape.
pub fn path() -> Option<PathBuf> {
    super::tape::app_dir().map(|d| d.join(FILE))
}

/// Writes `kept` to `path`, making its folder if need be.
///
/// # Errors
///
/// If the folder or the file cannot be written.
pub fn save(path: &Path, kept: &Kept) -> Result<(), String> {
    if let Some(dir) = path.parent() {
        std::fs::create_dir_all(dir)
            .map_err(|e| format!("cannot create {}: {e}", dir.display()))?;
    }
    std::fs::write(path, kept.text()).map_err(|e| format!("cannot write {}: {e}", path.display()))
}

#[cfg(test)]
mod tests {
    use super::*;

    /// A table of eight entries, AAA down to HHH, scores 8000 down to 1000.
    fn shipped() -> Vec<u8> {
        (0..HEROES)
            .flat_map(|i| {
                let c = b'A' + i as u8;
                let mut e = vec![c, c, c];
                e.extend_from_slice(format!("00{}000", 8 - i).as_bytes());
                e.push(10 * i as u8);
                e
            })
            .collect()
    }

    /// `table` with an entry put in at `at`, as the game does.
    fn with_entry(table: &[u8], at: usize, name: &[u8; 3]) -> Vec<u8> {
        let mut t = table[..at * ENTRY].to_vec();
        t.extend_from_slice(name);
        t.extend_from_slice(b"005500");
        t.push(42);
        t.extend_from_slice(&table[at * ENTRY..(HEROES - 1) * ENTRY]);
        t
    }

    const PLAYED: Record = Record {
        highest: 3,
        training: false,
        switches: starquake::game::Training([false; 5]),
    };

    #[test]
    fn the_file_holds_the_table_and_its_levels() {
        let mut keeper = Keeper::new(None, &shipped());
        let now = with_entry(&shipped(), 3, b"J N");
        let saved = keeper.heroes(&now, Some(3), PLAYED).unwrap().clone();
        assert_eq!(saved.levels[3], Some(3));
        assert_eq!(saved.levels[2], None, "the tape's own");
        let read = Kept::parse(&saved.text()).unwrap();
        assert_eq!(read, saved, "{}", saved.text());
        assert_eq!(&read.name(3), b"J N", "a space in the initials");
    }

    #[test]
    fn levels_move_down_with_their_entries() {
        let mut keeper = Keeper::new(None, &shipped());
        let first = with_entry(&shipped(), 5, b"ONE");
        keeper.heroes(&first, Some(5), PLAYED);
        let second = with_entry(&first, 1, b"TWO");
        let top = Record {
            highest: (LEVELS.len() - 1) as u8,
            training: false,
            switches: starquake::game::Training::default(),
        };
        let saved = keeper.heroes(&second, Some(1), top).unwrap().clone();
        assert_eq!(saved.levels[1], Some(top.highest));
        assert_eq!(saved.levels[6], Some(3), "ONE moved down one");
        assert!(
            Kept::parse(&saved.text()).is_some(),
            "the top level reads back"
        );
    }

    #[test]
    fn a_game_with_training_is_shown_and_then_put_back() {
        let mut keeper = Keeper::new(None, &shipped());
        let now = with_entry(&shipped(), 0, b"TRN");
        let training = Record {
            highest: 0,
            training: true,
            switches: starquake::game::Training([true, false, false, false, false]),
        };
        assert!(
            keeper.heroes(&now, Some(0), training).is_none(),
            "not saved"
        );
        assert_eq!(keeper.shown(), Some(shipped()), "put back");
        assert_eq!(keeper.shown(), None);
    }

    #[test]
    fn a_damaged_file_is_left_alone() {
        let mut keeper = Keeper::new(Some("not a table"), &shipped());
        assert_eq!(keeper.kept.table, shipped(), "the tape's own table");
        let now = with_entry(&shipped(), 0, b"NEW");
        assert!(
            keeper.heroes(&now, Some(0), PLAYED).is_none(),
            "never written"
        );
    }

    #[test]
    fn a_file_with_a_level_this_program_does_not_have_is_not_a_table() {
        let mut text = Kept::from_tape(&shipped()).text();
        text = text.replacen(" -\n", " 99\n", 1);
        assert!(Kept::parse(&text).is_none());
    }
}
