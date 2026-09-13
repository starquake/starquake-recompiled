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
/// walls inside it between openings that do not reach each other.
///
/// An opening is where he can reach the edge, not a promise that he can get
/// there: a gap high in a wall can need a lift, and a door can be shut.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub struct Openings {
    pub left: bool,
    pub right: bool,
    pub up: bool,
    pub down: bool,
    /// Walls inside the room, each from its centre out to a place on its
    /// edge; unused slots are `None`.
    pub walls: [Option<Wall>; 8],
}

/// A wall inside a room, from its centre out to `to`, a place on the edge
/// measured clockwise from the top-left corner in cells (up to [`AROUND`]).
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct Wall {
    pub to: u8,
    /// Whether a security door is all that divides the room here, which the
    /// right item lets BLOB through.
    pub door: bool,
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
        let display = &self.display;
        Room {
            openings: scan(|row, col| free(display.attr(row, col))),
            shut: Parts::find(|row, col| free(display.attr(row, col))),
            open: Parts::find(|row, col| free(display.attr(row, col)) || in_door(row, col)),
            passage,
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
            o.walls = walls(&ports(room, left, right));
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
/// wherever it is. Not where it is carried or delivered, though: an item's
/// row is 1 while it is being picked up and 2 to 5 in the inventory, and a
/// delivered one is parked in the core room at row 10. An item not yet
/// placed in its room has row 0 and still counts; its room is known.
fn missing_piece_rooms(core_slots: &[u8; 9], items: &[Item]) -> RoomSet {
    let wanted = |graphic: u8| {
        core_slots
            .iter()
            .any(|&slot| slot & 0x80 != 0 && slot & 0x7F == graphic)
    };
    let mut rooms = RoomSet::default();
    for item in items {
        let carried = (1..=5).contains(&item.row());
        let delivered = item.room() == crate::cores::CORE_ROOM && item.row() == 0x0A;
        if wanted(item.graphic()) && !carried && !delivered {
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
        walls: [None; 8],
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

/// The walls inside a room with these ports: none when every port is in one
/// part, and otherwise one from the centre to halfway between each pair of
/// neighbouring ports in different parts. A wall is a door's when opening
/// the doors joins the two.
fn walls(ports: &[Port]) -> [Option<Wall>; 8] {
    let mut walls = [None; 8];
    let first = ports.first().map(|p| p.shut);
    if ports.iter().all(|p| Some(p.shut) == first) {
        return walls;
    }
    let n = ports.len();
    let apart = (0..n).filter_map(|i| {
        let (a, b) = (ports[i], ports[(i + 1) % n]);
        (a.shut != b.shut).then(|| {
            let gap = (b.start + AROUND - a.end) % AROUND;
            Wall {
                to: (a.end + gap / 2) % AROUND,
                door: a.open == b.open,
            }
        })
    });
    for (slot, wall) in walls.iter_mut().zip(apart) {
        *slot = Some(wall);
    }
    walls
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

    /// The walls of a room drawn as text, with `D` for a door's cells:
    /// solid while shut.
    fn walls_of(lines: &[String]) -> Vec<Wall> {
        let open_lines: Vec<String> = lines.iter().map(|l| l.replace('D', ".")).collect();
        let open_refs: Vec<&str> = open_lines.iter().map(String::as_str).collect();
        let shut_lines: Vec<String> = lines.iter().map(|l| l.replace('D', "#")).collect();
        let shut_refs: Vec<&str> = shut_lines.iter().map(String::as_str).collect();
        let r = Room {
            openings: scan(room(&shut_refs)),
            shut: Parts::find(room(&shut_refs)),
            open: Parts::find(room(&open_refs)),
            passage: None,
        };
        walls(&ports(&r, false, false))
            .into_iter()
            .flatten()
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
        let items = [
            item(50, 1, 0x09),                         // being picked up
            item(51, 2, 0x09),                         // in the inventory
            item(52, 5, 0x09),                         // last inventory slot
            item(crate::cores::CORE_ROOM, 0x0A, 0x09), // delivered
        ];
        assert_eq!(missing_piece_rooms(&slots, &items), RoomSet::default());
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
        assert!(walls_of(&open_both_sides()).is_empty());
    }

    #[test]
    fn a_wall_down_the_middle_is_a_line_from_top_to_bottom() {
        let mut lines = open_both_sides();
        for line in &mut lines {
            line.replace_range(15..17, "##");
        }
        let walls = walls_of(&lines);
        // Halfway along the closed top and the closed bottom.
        let tos: Vec<u8> = walls.iter().map(|w| w.to).collect();
        assert_eq!(tos.len(), 2, "{walls:?}");
        assert!(tos.iter().any(|&t| t < TOP), "one to the top: {tos:?}");
        assert!(
            tos.iter()
                .any(|&t| (TOP + SIDE..2 * TOP + SIDE).contains(&t)),
            "one to the bottom: {tos:?}"
        );
        assert!(walls.iter().all(|w| !w.door));
    }

    #[test]
    fn a_door_in_the_dividing_wall_makes_it_a_door() {
        let mut lines = open_both_sides();
        for (r, line) in lines.iter_mut().enumerate() {
            let cells = if (7..10).contains(&r) { "DD" } else { "##" };
            line.replace_range(15..17, cells);
        }
        let walls = walls_of(&lines);
        assert_eq!(walls.len(), 2);
        assert!(walls.iter().all(|w| w.door));
    }

    #[test]
    fn a_gap_that_goes_round_the_wall_joins_the_parts() {
        let mut lines = open_both_sides();
        for (r, line) in lines.iter_mut().enumerate() {
            if r != 1 && r != 2 {
                line.replace_range(15..17, "##");
            }
        }
        assert!(walls_of(&lines).is_empty());
    }
}
