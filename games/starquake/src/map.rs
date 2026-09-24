//! The planet as a map, for the guidance panel: which edges of each room can
//! be left through and where a room is divided inside (#2), and which rooms
//! hold the core pieces still needed (#3).
//!
//! Walking off a room's left or right edge moves one room along, and off its
//! top or bottom sixteen (`Game::room_exit`), so the 512 rooms are a grid 16
//! wide and 32 tall and a room's number is its place on it.
//!
//! Nothing here changes the game. To find the openings, each room is built
//! into a copy of it and its cells read back.

use crate::game::Game;
use crate::pickups::{Item, RoomSet};

/// The map's width and height, in rooms.
pub const COLS: u16 = 16;
pub const ROWS: u16 = 32;

/// The first and last character rows the room occupies on screen, and its
/// last column.
const FIRST_ROW: u8 = crate::room::TOP_ROW;
const LAST_ROW: u8 = 23;
const LAST_COL: u8 = 31;

/// Where BLOB's top-left cell can be: he is two cells by two, so one row and
/// one column short of the room.
const SPOTS_DOWN: usize = (LAST_ROW - FIRST_ROW) as usize;
const SPOTS_ACROSS: usize = LAST_COL as usize;

/// The length of each edge in cells, clockwise from the top-left corner:
/// top, right, bottom, left. A place on the edge is measured in the same
/// cells, from 0 up to [`AROUND`].
const TOP: u8 = LAST_COL + 1;
const SIDE: u8 = LAST_ROW - FIRST_ROW + 1;
pub const AROUND: u8 = 2 * (TOP + SIDE);

/// Which edges of a room have an opening, a gap BLOB fits through, and the
/// walls inside it between openings that do not reach each other (#92).
///
/// An opening is where he can reach the edge, not a promise that he can get
/// there: a gap high in a wall can need a lift, and a door can be shut.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub struct Openings {
    pub left: bool,
    pub right: bool,
    pub up: bool,
    pub down: bool,
    /// Walls inside the room, where they stand: the solid cells between
    /// openings that do not reach each other.
    pub divides: Divides,
}

/// The solid cells that keep two of a room's openings apart, a bit per cell
/// (bit `col` of `cells[row]`, rows from the top of the play area), and
/// which of them are a door's or a teleport pad's: the two parts they keep
/// apart become one when doors and pads are open (#92).
///
/// They are found by growing every part that has an opening outwards
/// through the room, free cells and solid alike, one cell a step: a solid
/// cell reached from two different parts at the same time, or next to a
/// cell reached from another part, lies between them. A part with no
/// opening of its own, a sealed pocket, grows nothing and draws nothing.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub struct Divides {
    pub cells: [u32; 18],
    pub doors: [u32; 18],
}

impl Divides {
    /// Whether the cell at (`row`, `col`) of the play area is a wall.
    pub fn wall(&self, row: usize, col: usize) -> bool {
        row < 18 && col < 32 && self.cells[row] & (1 << col) != 0
    }

    /// Whether the cell at (`row`, `col`) is a door's or a pad's.
    pub fn door(&self, row: usize, col: usize) -> bool {
        row < 18 && col < 32 && self.doors[row] & (1 << col) != 0
    }

    /// Finds the walls of `room` between the parts in `ported`, those with
    /// an opening.
    fn find(room: &Room, ported: &[u8]) -> Divides {
        // Every cell BLOB stands on in a ported part is a source; each cell
        // then remembers the nearest part, by breadth-first growth.
        let mut nearest = [[0u8; 32]; 18];
        let mut queue = std::collections::VecDeque::new();
        for r in 0..SPOTS_DOWN {
            for c in 0..SPOTS_ACROSS {
                let p = room.shut.at(FIRST_ROW + r as u8, c as u8);
                if p == 0 || !ported.contains(&p) {
                    continue;
                }
                for (rr, cc) in [(r, c), (r, c + 1), (r + 1, c), (r + 1, c + 1)] {
                    if nearest[rr][cc] == 0 {
                        nearest[rr][cc] = p;
                        queue.push_back((rr, cc));
                    }
                }
            }
        }
        // A solid cell on the seam, reached by two parts, is recorded as a
        // wall; a door's when opening doors and pads joins the two.
        let mut cells = [0u32; 18];
        let mut doors = [0u32; 18];
        let open_of = |p: u8| {
            (0..SPOTS_DOWN)
                .flat_map(|r| (0..SPOTS_ACROSS).map(move |c| (r, c)))
                .find(|&(r, c)| room.shut.at(FIRST_ROW + r as u8, c as u8) == p)
                .map_or(0, |(r, c)| room.open.at(FIRST_ROW + r as u8, c as u8))
        };
        let mut mark = |r: usize, c: usize, a: u8, b: u8| {
            if room.solid[r] & (1 << c) == 0 {
                return;
            }
            cells[r] |= 1 << c;
            if open_of(a) == open_of(b) {
                doors[r] |= 1 << c;
            }
        };
        while let Some((r, c)) = queue.pop_front() {
            let here = nearest[r][c];
            let beside = [
                (r, c + 1),
                (r + 1, c),
                (r, c.wrapping_sub(1)),
                (r.wrapping_sub(1), c),
            ];
            for (rr, cc) in beside {
                if rr >= 18 || cc >= 32 {
                    continue;
                }
                let there = nearest[rr][cc];
                if there == 0 {
                    nearest[rr][cc] = here;
                    queue.push_back((rr, cc));
                } else if there != here {
                    mark(r, c, here, there);
                    mark(rr, cc, here, there);
                }
            }
        }
        Divides { cells, doors }
    }
}

/// Whether a cell's attribute lets BLOB through: below `0x40` is solid, as
/// `Game::collide` has it.
fn free(attr: u8) -> bool {
    attr >= 0x40
}

/// The marker a wall passage leaves: touching it while walking left or right
/// takes BLOB into the room beside, to that room's own passage marker.
const PASSAGE: u8 = 0x0F;

/// The marker a security door's tile leaves (`Game::security_door`).
const DOOR: u8 = 0;

/// The parts of a room BLOB can move between, ignoring gravity: every place
/// his top-left cell fits is numbered by the part it is in, and 0 where he
/// does not fit. Two places in the same part are joined by free cells, so a
/// solid wall between them gives them different numbers.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct Parts([[u8; SPOTS_ACROSS]; SPOTS_DOWN]);

impl Parts {
    /// Numbers the parts of a room from `free(row, col)`.
    fn find(free: impl Fn(u8, u8) -> bool) -> Parts {
        let fits = |r: usize, c: usize| {
            let (row, col) = (FIRST_ROW + r as u8, c as u8);
            free(row, col) && free(row, col + 1) && free(row + 1, col) && free(row + 1, col + 1)
        };
        let mut parts = [[0u8; SPOTS_ACROSS]; SPOTS_DOWN];
        let mut next = 0;
        for r in 0..SPOTS_DOWN {
            for c in 0..SPOTS_ACROSS {
                if parts[r][c] != 0 || !fits(r, c) {
                    continue;
                }
                next += 1;
                parts[r][c] = next;
                let mut todo = vec![(r, c)];
                while let Some((r, c)) = todo.pop() {
                    let beside = [
                        (r, c + 1),
                        (r + 1, c),
                        (r, c.wrapping_sub(1)),
                        (r.wrapping_sub(1), c),
                    ];
                    for (r, c) in beside {
                        if r < SPOTS_DOWN && c < SPOTS_ACROSS && parts[r][c] == 0 && fits(r, c) {
                            parts[r][c] = next;
                            todo.push((r, c));
                        }
                    }
                }
            }
        }
        Parts(parts)
    }

    /// Joins the parts on either side of a tile `width` cells wide and
    /// three tall from its cell at screen (`row`, `col`) (#92): what a door
    /// does when it opens, moving BLOB 48 pixels past the tile rather than
    /// through its cells, and what a teleport pad does when it is blanked.
    /// So on each side the nearest place BLOB fits within six cells counts,
    /// as ZX Sidekick found walking him through them.
    fn join_across(&mut self, row: u8, col: u8, width: u8) {
        const REACH: u8 = 6;
        let rows = row.saturating_sub(1)..row + 3;
        // BLOB's top-left cell beside the tile: two cells to its left, so
        // his right cell touches it, or just past its right side; then
        // further out.
        let left = (0..REACH).map(|d| col.wrapping_sub(2 + d));
        let right = (0..REACH).map(|d| col + width + d);
        let mut parts: Vec<u8> = Vec::new();
        for side in [left.collect::<Vec<u8>>(), right.collect()] {
            let nearest = side
                .into_iter()
                .find_map(|c| rows.clone().map(|r| self.at(r, c)).find(|&p| p != 0));
            if let Some(p) = nearest
                && !parts.contains(&p)
            {
                parts.push(p);
            }
        }
        if let Some((&first, rest)) = parts.split_first() {
            for line in &mut self.0 {
                for cell in line.iter_mut() {
                    if rest.contains(cell) {
                        *cell = first;
                    }
                }
            }
        }
    }

    /// The part of the room BLOB is in with his top-left cell at screen
    /// (`row`, `col`), or 0 where he does not fit.
    pub fn at(&self, row: u8, col: u8) -> u8 {
        let r = row.wrapping_sub(FIRST_ROW) as usize;
        self.0
            .get(r)
            .and_then(|cols| cols.get(col as usize))
            .copied()
            .unwrap_or(0)
    }
}

/// One room as the map reads it.
struct Room {
    openings: Openings,
    /// Its parts with security doors shut, and with them open.
    shut: Parts,
    open: Parts,
    /// The cell of its wall passage marker, if it has one.
    passage: Option<(u8, u8)>,
    /// Its solid cells, a bit per cell: bit `col` of `solid[row]`, rows
    /// from the top of the play area.
    solid: [u32; 18],
}

/// The character cell of a marker's position (`Game::add_marker`).
fn marker_cell(x: u8, y: u8) -> (u8, u8) {
    (0x18 - ((y as u16 + 1) >> 3) as u8, x >> 3)
}

impl Game {
    /// Builds `room` into this game, which must be a copy, and reads it.
    fn read_room(&mut self, room: u16) -> Room {
        self.room = room;
        self.display.clear_room_area();
        self.restore_mem.fill(0);
        self.restore_ptr = crate::room::RESTORE_START;
        self.build_room_tiles();
        let cell = |m: &crate::room::Marker| marker_cell(m.x, m.y);
        let markers = &self.objects.markers;
        let passage = markers.iter().find(|m| m.kind == PASSAGE).map(cell);
        // A door's tile is four cells by three, from its marker's cell.
        let doors: Vec<(u8, u8)> = markers
            .iter()
            .filter(|m| m.kind == DOOR)
            .map(cell)
            .collect();
        let in_door = |row: u8, col: u8| {
            doors
                .iter()
                .any(|&(r, c)| (r..r + 3).contains(&row) && (c..c + 4).contains(&col))
        };
        // A teleport pad, blanked when BLOB touches it carrying the pad key:
        // the cells `Game::blank_teleport_pad` clears.
        let pad = self
            .objects
            .teleport
            .map(|((col, row), _)| ((col & 0xFC) | 1, row));
        let in_pad =
            |row: u8, col: u8| pad.is_some_and(|(c, r)| c == col && (r..r + 3).contains(&row));
        let display = &self.display;
        let mut open = Parts::find(|row, col| {
            free(display.attr(row, col)) || in_door(row, col) || in_pad(row, col)
        });
        for &(row, col) in &doors {
            open.join_across(row, col, 4);
        }
        if let Some((col, row)) = pad {
            open.join_across(row, col, 1);
        }
        let mut solid = [0u32; 18];
        for (r, bits) in solid.iter_mut().enumerate() {
            for col in 0..32u8 {
                if !free(display.attr(FIRST_ROW + r as u8, col)) {
                    *bits |= 1 << col;
                }
            }
        }
        Room {
            openings: scan(|row, col| free(display.attr(row, col))),
            shut: Parts::find(|row, col| free(display.attr(row, col))),
            open,
            passage,
            solid,
        }
    }

    /// The parts of `room` BLOB can move between, with its security doors
    /// open, built into a copy of the game.
    pub fn room_parts(&self, room: u16) -> Parts {
        self.clone().read_room(room).open
    }

    /// The openings of every room, indexed by room number.
    ///
    /// An edge is open where there is a gap in it, or a passage through the
    /// wall: two rooms side by side that both have a passage marker. The core
    /// room opens only on its left, where it is entered.
    pub fn all_openings(&self) -> Vec<Openings> {
        let mut scratch = self.clone();
        let rooms: Vec<Room> = (0..COLS * ROWS).map(|r| scratch.read_room(r)).collect();
        let passage = |room: usize| rooms.get(room).is_some_and(|r| r.passage.is_some());
        let mut openings = Vec::with_capacity(rooms.len());
        for (i, room) in rooms.iter().enumerate() {
            let col = i as u16 % COLS;
            let right = col != COLS - 1 && passage(i) && passage(i + 1);
            let left = col != 0 && passage(i) && i > 0 && passage(i - 1);
            let mut o = room.openings;
            o.right |= right;
            o.left |= left;
            let ported: Vec<u8> = ports(room, left, right).iter().map(|p| p.shut).collect();
            o.divides = Divides::find(room, &ported);
            openings.push(o);
        }
        // The core room is not played in its tiles: walking in from the left
        // runs its own screen, which puts BLOB back in the room he came from
        // (`Game::core_room`). So its one way in and out is its left edge.
        openings[crate::cores::CORE_ROOM as usize] = Openings {
            left: true,
            ..Openings::default()
        };
        openings
    }

    /// The rooms holding a core piece the core still needs, for the guidance
    /// map (#3).
    pub fn missing_piece_rooms(&self) -> RoomSet {
        missing_piece_rooms(&self.core_slots, &self.items)
    }
}

/// The rooms of the items that would fill a hole still open in the core.
///
/// A hole is open while its slot has bit 7 set, and the rest of the byte is
/// the graphic of the piece that fills it: the core takes any carried item
/// with that graphic (`Game::core_room`), so every item with it counts,
/// wherever it is. An item's row is 1 while it is being picked up and 2 to 5
/// in the inventory, and a delivered one is parked in the core room at row
/// 10; neither is somewhere to go. An item not yet placed in its room has row
/// 0 and still counts; its room is known.
///
/// A hole whose piece is being carried marks nothing: most pieces come in
/// twos, and the other one is not needed while you have one.
fn missing_piece_rooms(core_slots: &[u8; 9], items: &[Item]) -> RoomSet {
    let carried = |item: &Item| (1..=5).contains(&item.row());
    let wanted = |graphic: u8| {
        core_slots
            .iter()
            .any(|&slot| slot & 0x80 != 0 && slot & 0x7F == graphic)
            && !items.iter().any(|i| carried(i) && i.graphic() == graphic)
    };
    let mut rooms = RoomSet::default();
    for item in items {
        let delivered = item.room() == crate::cores::CORE_ROOM && item.row() == 0x0A;
        if wanted(item.graphic()) && !carried(item) && !delivered {
            rooms.set(item.room(), true);
        }
    }
    rooms
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
        divides: Divides::default(),
    }
}

/// A stretch of a room's edge BLOB can leave through, from `start` to `end`
/// clockwise, and the part of the room it belongs to with doors shut and
/// with them open.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
struct Port {
    start: u8,
    end: u8,
    shut: u8,
    open: u8,
}

/// The stretches of a room's edge BLOB can leave through, in clockwise
/// order: every place he fits against an edge, joined where they touch and
/// belong to the same part, and the wall passages on the sides given.
fn ports(room: &Room, passage_left: bool, passage_right: bool) -> Vec<Port> {
    let last_down = (SPOTS_DOWN - 1) as u8;
    let last_across = (SPOTS_ACROSS - 1) as u8;
    let port = |start: u8, row: u8, col: u8| {
        let (shut, open) = (room.shut.at(row, col), room.open.at(row, col));
        (shut != 0).then_some(Port {
            start,
            end: start + 2,
            shut,
            open,
        })
    };
    let mut pieces: Vec<Port> = Vec::new();
    for c in 0..=last_across {
        pieces.extend(port(c, FIRST_ROW, c));
    }
    for r in 0..=last_down {
        pieces.extend(port(TOP + r, FIRST_ROW + r, last_across));
    }
    for c in (0..=last_across).rev() {
        pieces.extend(port(
            TOP + SIDE + (last_across - c),
            FIRST_ROW + last_down,
            c,
        ));
    }
    for r in (0..=last_down).rev() {
        pieces.extend(port(2 * TOP + SIDE + (last_down - r), FIRST_ROW + r, 0));
    }
    if let Some((row, col)) = room.passage {
        // The part the passage is in: the first place BLOB fits next to its
        // tile, which is four cells by three.
        let near = (row.saturating_sub(1)..row + 3)
            .flat_map(|r| (col.saturating_sub(2)..col + 4).map(move |c| (r, c)))
            .find(|&(r, c)| room.shut.at(r, c) != 0);
        if let Some((r, c)) = near {
            let at = row.saturating_sub(FIRST_ROW).min(SIDE - 3);
            let (shut, open) = (room.shut.at(r, c), room.open.at(r, c));
            if passage_right {
                pieces.push(Port {
                    start: TOP + at,
                    end: TOP + at + 3,
                    shut,
                    open,
                });
            }
            if passage_left {
                let start = 2 * TOP + SIDE + (SIDE - 3 - at);
                pieces.push(Port {
                    start,
                    end: start + 3,
                    shut,
                    open,
                });
            }
        }
    }
    pieces.sort_by_key(|p| p.start);
    let mut joined: Vec<Port> = Vec::new();
    for p in pieces {
        match joined.last_mut() {
            Some(last) if last.shut == p.shut && p.start <= last.end => {
                last.end = last.end.max(p.end);
            }
            _ => joined.push(p),
        }
    }
    joined
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

    /// A room's solid cells from its text, `#` solid.
    fn solid_of(lines: &[&str]) -> [u32; 18] {
        let mut solid = [0u32; 18];
        for (r, line) in lines.iter().enumerate() {
            for (c, ch) in line.chars().enumerate() {
                if ch == '#' {
                    solid[r] |= 1 << c;
                }
            }
        }
        solid
    }

    /// A room's divides from its text, with `D` a door's cells: solid while
    /// shut.
    fn divides_of(lines: &[String]) -> Divides {
        let open_lines: Vec<String> = lines.iter().map(|l| l.replace('D', ".")).collect();
        let open_refs: Vec<&str> = open_lines.iter().map(String::as_str).collect();
        let shut_lines: Vec<String> = lines.iter().map(|l| l.replace('D', "#")).collect();
        let shut_refs: Vec<&str> = shut_lines.iter().map(String::as_str).collect();
        let r = Room {
            openings: scan(room(&shut_refs)),
            shut: Parts::find(room(&shut_refs)),
            open: Parts::find(room(&open_refs)),
            passage: None,
            solid: solid_of(&shut_refs),
        };
        let ported: Vec<u8> = ports(&r, false, false).iter().map(|p| p.shut).collect();
        Divides::find(&r, &ported)
    }

    /// The wall cells of a divides map as text, `#` a wall, `D` a door's.
    fn draw_divides(d: &Divides) -> Vec<String> {
        (0..18)
            .map(|r| {
                (0..32)
                    .map(|c| {
                        if d.door(r, c) {
                            'D'
                        } else if d.wall(r, c) {
                            '#'
                        } else {
                            '.'
                        }
                    })
                    .collect()
            })
            .collect()
    }

    /// An item in `room` at `row`, with `graphic`.
    fn item(room: u16, row: u8, graphic: u8) -> Item {
        Item([
            0,
            ((room >> 8) as u8).rotate_right(1) | row,
            room as u8,
            graphic,
        ])
    }

    #[test]
    fn a_piece_for_an_open_hole_marks_its_room() {
        // Hole 0 wants graphic 0x09 and hole 1 is filled (it holds its own
        // number); the rest want graphics nothing here has.
        let mut slots = [0x80 | 0x30; 9];
        slots[0] = 0x80 | 0x09;
        slots[1] = 1;
        let items = [
            item(300, 0, 0x09), // not placed yet: counts
            item(40, 12, 0x09), // a second one with the same graphic
            item(41, 12, 0x01), // wanted by no open hole
            item(42, 12, 0x0F), // not a core piece at all
        ];
        let rooms = missing_piece_rooms(&slots, &items);
        assert!(rooms.contains(300) && rooms.contains(40));
        assert!(!rooms.contains(41) && !rooms.contains(42));
    }

    #[test]
    fn carried_and_delivered_pieces_are_not_marked() {
        let mut slots = [0x80 | 0x30; 9];
        slots[0] = 0x80 | 0x09;
        slots[1] = 0x80 | 0x0A;
        slots[2] = 0x80 | 0x0B;
        let items = [
            item(crate::cores::CORE_ROOM, 0x0A, 0x09), // delivered
            item(51, 2, 0x0A),                         // in the inventory
            item(52, 5, 0x0B),                         // last inventory slot
        ];
        assert_eq!(missing_piece_rooms(&slots, &items), RoomSet::default());
    }

    #[test]
    fn carrying_a_piece_unmarks_its_twin() {
        let mut slots = [0x80 | 0x30; 9];
        slots[0] = 0x80 | 0x09;
        let twin = item(60, 12, 0x09);
        assert!(missing_piece_rooms(&slots, &[twin]).contains(60));
        for row in [1, 2, 5] {
            let rooms = missing_piece_rooms(&slots, &[item(61, row, 0x09), twin]);
            assert!(!rooms.contains(60), "carried at row {row}");
        }
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

    /// A room open on the left and right, rows 8 and 9 of the room.
    fn open_both_sides() -> Vec<String> {
        let mut lines = walled();
        for r in [8, 9] {
            lines[r].replace_range(0..1, ".");
            lines[r].replace_range(31..32, ".");
        }
        lines
    }

    #[test]
    fn a_room_in_one_part_has_no_walls_inside() {
        let d = divides_of(&open_both_sides());
        assert_eq!(d, Divides::default(), "{:?}", draw_divides(&d));
    }

    #[test]
    fn a_wall_down_the_middle_stands_where_it_is() {
        let mut lines = open_both_sides();
        for line in &mut lines {
            line.replace_range(15..17, "##");
        }
        let d = divides_of(&lines);
        // The two solid columns between the halves, and nothing else: not
        // the outer walls, which keep no two openings apart.
        for (r, line) in draw_divides(&d).iter().enumerate() {
            let expected: String = (0..32)
                .map(|c| if (15..17).contains(&c) { '#' } else { '.' })
                .collect();
            assert_eq!(line, &expected, "row {r}");
        }
        assert!(d.doors.iter().all(|&bits| bits == 0));
    }

    #[test]
    fn a_door_in_the_dividing_wall_makes_it_a_door_s() {
        let mut lines = open_both_sides();
        for (r, line) in lines.iter_mut().enumerate() {
            let cells = if (7..10).contains(&r) { "DD" } else { "##" };
            line.replace_range(15..17, cells);
        }
        let drawn = draw_divides(&divides_of(&lines));
        assert!(drawn.iter().all(|l| !l.contains('#')), "{drawn:?}");
        assert!(drawn.iter().any(|l| l.contains('D')), "{drawn:?}");
    }

    #[test]
    fn a_door_joins_the_parts_either_side_of_its_tile_though_not_through_it() {
        // A wall four cells thick down the middle: opening the door's tile
        // (its cells) would not join the halves, walking through it does.
        let mut lines = open_both_sides();
        for line in &mut lines {
            line.replace_range(14..18, "####");
        }
        let refs: Vec<&str> = lines.iter().map(String::as_str).collect();
        let mut parts = Parts::find(room(&refs));
        let (row, left, right) = (FIRST_ROW + 8, 5, 25);
        assert_ne!(parts.at(row, left), parts.at(row, right));
        parts.join_across(FIRST_ROW + 7, 14, 4);
        assert_eq!(parts.at(row, left), parts.at(row, right));
    }

    #[test]
    fn a_pocket_with_no_opening_draws_no_wall() {
        // A sealed pocket in the top-right corner, beside a room open both
        // sides.
        let mut lines = open_both_sides();
        for line in &mut lines[1..5] {
            line.replace_range(24..25, "#");
        }
        lines[5].replace_range(24..31, "#######");
        let d = divides_of(&lines);
        assert_eq!(d, Divides::default(), "{:?}", draw_divides(&d));
    }

    #[test]
    fn a_gap_that_goes_round_the_wall_joins_the_parts() {
        let mut lines = open_both_sides();
        for (r, line) in lines.iter_mut().enumerate() {
            if r != 1 && r != 2 {
                line.replace_range(15..17, "##");
            }
        }
        assert_eq!(divides_of(&lines), Divides::default());
    }
}
