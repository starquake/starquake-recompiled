//! Keeping every teleport code a player has discovered, between games and
//! runs (#115): each booth walked into, which shows its own code, and each
//! code typed right in a booth, which the game hands to the host as the
//! booth accepts it ([`starquake::host::Host::teleported`]). A code names a
//! teleport, and the fifteen are the same in every game, so a code once
//! found stays right. This keeps them in `teleporter-codes.txt` beside the
//! kept tape, and level 1 lists them from the start of every game with the
//! ones seen in it.

use std::path::{Path, PathBuf};

use starquake::game::SeenTeleporter;

const FILE: &str = "teleporter-codes.txt";

/// The codes kept, in the order they were first found.
#[derive(Clone, Debug, Default, PartialEq, Eq)]
pub struct Kept {
    pub codes: Vec<SeenTeleporter>,
}

impl Kept {
    /// The file's text: a line a code, its five letters and the room of the
    /// teleport it names.
    pub fn text(&self) -> String {
        self.codes
            .iter()
            .map(|t| format!("{} {}\n", String::from_utf8_lossy(&t.code), t.room))
            .collect()
    }

    /// The codes in the file's text. A line that is not one is left out,
    /// so a file edited by hand loses only what it broke.
    pub fn parse(text: &str) -> Kept {
        let codes = text
            .lines()
            .filter_map(|line| {
                let (code, room) = line.trim().split_once(' ')?;
                let code: [u8; 5] = code.as_bytes().try_into().ok()?;
                code.iter().all(u8::is_ascii_graphic).then_some(())?;
                let room = room.trim().parse::<u16>().ok()?;
                Some(SeenTeleporter { room, code })
            })
            .collect();
        Kept { codes }
    }

    /// Adds a code found. Returns whether it is new, which is when the file
    /// needs writing.
    pub fn add(&mut self, room: u16, code: [u8; 5]) -> bool {
        if self.codes.iter().any(|t| t.room == room) {
            return false;
        }
        self.codes.push(SeenTeleporter { room, code });
        true
    }

    /// The codes to list: those seen in this game, in their order, then the
    /// kept ones not among them.
    pub fn with_seen(&self, seen: &[SeenTeleporter]) -> Vec<SeenTeleporter> {
        let mut all = seen.to_vec();
        all.extend(
            self.codes
                .iter()
                .filter(|k| !seen.iter().any(|s| s.room == k.room)),
        );
        all
    }
}

/// Where the codes are kept: beside the kept tape.
pub fn path() -> Option<PathBuf> {
    super::tape::app_dir().map(|d| d.join(FILE))
}

/// The codes kept at `path`, or none when there is no file yet.
pub fn load(path: Option<&Path>) -> Kept {
    path.and_then(|p| std::fs::read_to_string(p).ok())
        .map_or_else(Kept::default, |text| Kept::parse(&text))
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

    #[test]
    fn the_file_holds_each_code_and_its_room() {
        let mut kept = Kept::default();
        assert!(kept.add(190, *b"ABCDE"));
        assert!(kept.add(252, *b"FGHIJ"));
        assert_eq!(kept.text(), "ABCDE 190\nFGHIJ 252\n");
        assert_eq!(Kept::parse(&kept.text()), kept);
    }

    #[test]
    fn a_code_is_kept_once() {
        let mut kept = Kept::default();
        assert!(kept.add(190, *b"ABCDE"));
        assert!(!kept.add(190, *b"ABCDE"), "not new");
        assert_eq!(kept.codes.len(), 1);
    }

    #[test]
    fn a_broken_line_loses_only_itself() {
        let kept = Kept::parse("ABCDE 190\nABC 1\nFGHIJ x\n\nKLMNO 300\n");
        let rooms: Vec<u16> = kept.codes.iter().map(|t| t.room).collect();
        assert_eq!(rooms, [190, 300]);
    }

    #[test]
    fn seen_codes_come_first_and_kept_ones_are_not_listed_twice() {
        let mut kept = Kept::default();
        kept.add(190, *b"ABCDE");
        kept.add(252, *b"FGHIJ");
        let seen = [SeenTeleporter {
            room: 252,
            code: *b"FGHIJ",
        }];
        let all = kept.with_seen(&seen);
        let rooms: Vec<u16> = all.iter().map(|t| t.room).collect();
        assert_eq!(rooms, [252, 190]);
    }

    #[test]
    fn no_file_is_no_codes() {
        assert_eq!(load(None), Kept::default());
        let missing = std::env::temp_dir().join("sq-no-such-codes.txt");
        assert_eq!(load(Some(&missing)), Kept::default());
    }
}
