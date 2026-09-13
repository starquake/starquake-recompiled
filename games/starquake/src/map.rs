//! The planet as a map: which edges of each room can be left through, for
//! the guidance map (#2).
//!
//! Walking off a room's left or right edge moves one room along, and off its
//! top or bottom sixteen (`Game::room_exit`), so the 512 rooms are a grid 16
//! wide and 32 tall and a room's number is its place on it.
//!
//! Nothing here changes the game. Each room is built into a copy of it and
//! its cells read back.

use crate::game::Game;

/// The map's width and height, in rooms.
pub const COLS: u16 = 16;
pub const ROWS: u16 = 32;

/// The first and last character rows the room occupies on screen, and its
/// last column.
const FIRST_ROW: u8 = crate::room::TOP_ROW;
const LAST_ROW: u8 = 23;
const LAST_COL: u8 = 31;

/// Which edges of a room have an opening: a gap BLOB fits through.
///
/// An opening is where he can reach the edge, not a promise that he can get
/// there: a gap high in a wall can need a lift, and a door can be shut.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub struct Openings {
    pub left: bool,
    pub right: bool,
    pub up: bool,
    pub down: bool,
}

/// Whether a cell's attribute lets BLOB through: below `0x40` is solid, as
/// `Game::collide` has it.
fn free(attr: u8) -> bool {
    attr >= 0x40
}

/// The marker a wall passage leaves: touching it while walking left or right
/// takes BLOB into the room beside, to that room's own passage marker.
const PASSAGE: u8 = 0x0F;

impl Game {
    /// The openings of every room, indexed by room number.
    ///
    /// An edge is open where there is a gap in it, or a passage through the
    /// wall: two rooms side by side that both have a passage marker.
    pub fn all_openings(&self) -> Vec<Openings> {
        let mut scratch = self.clone();
        let mut passage = Vec::new();
        let mut openings: Vec<Openings> = (0..COLS * ROWS)
            .map(|room| {
                scratch.room = room;
                scratch.display.clear_room_area();
                scratch.restore_mem.fill(0);
                scratch.restore_ptr = crate::room::RESTORE_START;
                scratch.build_room_tiles();
                passage.push(scratch.objects.markers.iter().any(|m| m.kind == PASSAGE));
                scan(|row, col| free(scratch.display.attr(row, col)))
            })
            .collect();
        for room in 0..openings.len() - 1 {
            if room as u16 % COLS != COLS - 1 && passage[room] && passage[room + 1] {
                openings[room].right = true;
                openings[room + 1].left = true;
            }
        }
        openings
    }
}

/// Reads the four edges of a room from `free(row, col)`.
///
/// BLOB is two cells by two. `room_exit` sends him through the left edge
/// from columns 0–1, the right from 30–31, the top once he rises past the
/// top two rows and the bottom from the bottom two. So an edge is open where
/// a two-by-two window of free cells touches it.
fn scan(free: impl Fn(u8, u8) -> bool) -> Openings {
    let window = |row: u8, col: u8| {
        free(row, col) && free(row, col + 1) && free(row + 1, col) && free(row + 1, col + 1)
    };
    Openings {
        left: (FIRST_ROW..LAST_ROW).any(|row| window(row, 0)),
        right: (FIRST_ROW..LAST_ROW).any(|row| window(row, LAST_COL - 1)),
        up: (0..LAST_COL).any(|col| window(FIRST_ROW, col)),
        down: (0..LAST_COL).any(|col| window(LAST_ROW - 1, col)),
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    /// A room drawn as text, top row first: `#` solid, anything else free.
    fn room(lines: &[&str]) -> impl Fn(u8, u8) -> bool {
        let grid: Vec<Vec<bool>> = lines
            .iter()
            .map(|l| l.chars().map(|c| c != '#').collect())
            .collect();
        move |row, col| grid[(row - FIRST_ROW) as usize][col as usize]
    }

    fn walled() -> Vec<String> {
        (0..18)
            .map(|r| {
                if r == 0 || r == 17 {
                    "#".repeat(32)
                } else {
                    format!("#{}#", ".".repeat(30))
                }
            })
            .collect()
    }

    #[test]
    fn a_closed_room_has_no_openings() {
        let lines = walled();
        let refs: Vec<&str> = lines.iter().map(String::as_str).collect();
        assert_eq!(scan(room(&refs)), Openings::default());
    }

    #[test]
    fn a_gap_two_cells_wide_is_an_opening() {
        let mut lines = walled();
        // Two rows free in the left wall, and two columns in the floor.
        for r in [8, 9] {
            lines[r].replace_range(0..1, ".");
        }
        lines[17].replace_range(10..12, "..");
        let refs: Vec<&str> = lines.iter().map(String::as_str).collect();
        let o = scan(room(&refs));
        assert!(o.left && o.down);
        assert!(!o.right && !o.up);
    }

    #[test]
    fn a_gap_one_cell_wide_is_not() {
        let mut lines = walled();
        lines[8].replace_range(31..32, ".");
        lines[0].replace_range(5..6, ".");
        let refs: Vec<&str> = lines.iter().map(String::as_str).collect();
        assert_eq!(scan(room(&refs)), Openings::default());
    }
}
