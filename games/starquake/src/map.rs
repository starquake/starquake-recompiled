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
    /// The cell of its security door's tile, if it has one, rows from the
    /// top of the play area: where the map numbers the door (#94).
    pub door: Option<(u8, u8)>,
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

/// The marker a teleporter booth's tile leaves (`Game::teleport_booth`).
const BOOTH: u8 = 0x0D;

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
    /// Which way its passage leads: right when BLOB can stand beside the
    /// tile on its left and walk into it, left when he can on its right
    /// (#52).
    passage_right: bool,
    passage_left: bool,
    /// Its vacuum tube cells, a bit per cell as `solid`: a tube only ever
    /// carries BLOB up.
    lift: [u32; 18],
    /// The part (doors shut) BLOB can reach its passage from, and the part
    /// its teleporter booth is in; 0 for none.
    passage_part: u8,
    booth_part: u8,
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
        let mut lift = [0u32; 18];
        for r in 0..18 {
            for col in 0..32u8 {
                let a = display.attr(FIRST_ROW + r as u8, col);
                if !free(a) {
                    solid[r] |= 1 << col;
                }
                // The cell standing on which lifts BLOB (`blob_control`).
                if a & 0x7F == crate::blob::SPECIAL_CELL {
                    lift[r] |= 1 << col;
                }
            }
        }
        let shut = Parts::find(|row, col| free(display.attr(row, col)));
        // Where BLOB can stand beside the passage's tile, four cells wide
        // and three tall: with his top-left cell two columns left of it, or
        // just past its right side, on a row that overlaps it.
        let beside = |col: i16| {
            passage.is_some_and(|(row, _)| {
                (0..SPOTS_ACROSS as i16).contains(&col)
                    && (row.saturating_sub(1)..=row + 1).any(|r| shut.at(r, col as u8) != 0)
            })
        };
        let at = passage.map_or(0, |(_, col)| i16::from(col));
        let (passage_right, passage_left) = (beside(at - 2), beside(at + 4));
        let passage_part = passage.map_or(0, |(row, _)| {
            let side = if passage_right { at - 2 } else { at + 4 };
            (row.saturating_sub(1)..=row + 1)
                .map(|r| shut.at(r, side.clamp(0, 30) as u8))
                .find(|&p| p != 0)
                .unwrap_or(0)
        });
        // The part a tile's marker stands in: BLOB on it, his top cell on
        // the marker's row or up to two above, at its column or beside.
        let part_at_marker = |(row, col): (u8, u8)| {
            (0..=2u8)
                .flat_map(|up| [0, 1, -1i16].map(move |d| (row.saturating_sub(up), d)))
                .map(|(r, d)| shut.at(r, (i16::from(col) + d).clamp(0, 30) as u8))
                .find(|&p| p != 0)
                .unwrap_or(0)
        };
        let booth_part = self
            .objects
            .markers
            .iter()
            .find(|m| m.kind == BOOTH)
            .map_or(0, |m| part_at_marker(marker_cell(m.x, m.y)));
        let mut openings = scan(|row, col| free(display.attr(row, col)));
        openings.door = doors.first().map(|&(r, c)| (r - FIRST_ROW, c));
        Room {
            openings,
            shut,
            open,
            passage,
            solid,
            passage_right,
            passage_left,
            lift,
            passage_part,
            booth_part,
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

    /// The core pieces still needed that lie out on the planet, for level
    /// 5's routes (#52): the items [`Game::missing_piece_rooms`] marks.
    pub fn missing_pieces(&self) -> Vec<Item> {
        missing_pieces(&self.core_slots, &self.items)
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
    let mut rooms = RoomSet::default();
    for item in missing_pieces(core_slots, items) {
        rooms.set(item.room(), true);
    }
    rooms
}

/// The items [`missing_piece_rooms`] marks the rooms of.
fn missing_pieces(core_slots: &[u8; 9], items: &[Item]) -> Vec<Item> {
    let carried = |item: &Item| (1..=5).contains(&item.row());
    let wanted = |graphic: u8| {
        core_slots
            .iter()
            .any(|&slot| slot & 0x80 != 0 && slot & 0x7F == graphic)
            && !items.iter().any(|i| carried(i) && i.graphic() == graphic)
    };
    items
        .iter()
        .filter(|item| {
            let delivered = item.room() == crate::cores::CORE_ROOM && item.row() == 0x0A;
            wanted(item.graphic()) && !carried(item) && !delivered
        })
        .copied()
        .collect()
}

/// One step of a route (#52): into `room`, walking or by teleporter.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct Step {
    pub room: u16,
    pub teleport: bool,
}

/// Where BLOB can be on the planet: a room, and the part of it he is in,
/// with its doors and teleport pads open (#52).
pub type Place = (u16, u8);

/// The part with doors open that holds the part `shut` of a room with
/// doors shut.
fn open_part(room: &Room, shut: u8) -> u8 {
    (0..SPOTS_DOWN)
        .flat_map(|r| (0..SPOTS_ACROSS).map(move |c| (r, c)))
        .map(|(r, c)| (FIRST_ROW + r as u8, c as u8))
        .find(|&(r, c)| room.shut.at(r, c) == shut && shut != 0)
        .map_or(0, |(r, c)| room.open.at(r, c))
}

/// The part at the screen cell (`row`, `col`), or the one beside it when
/// the cell is one BLOB does not fit at; 0 when none is found.
fn part_near(parts: &Parts, row: u8, col: u8) -> u8 {
    let (row, col) = (i16::from(row), i16::from(col));
    [
        (0, 0),
        (0, 1),
        (1, 0),
        (1, 1),
        (-1, 0),
        (-1, 1),
        (0, -1),
        (1, -1),
    ]
    .into_iter()
    .map(|(dr, dc)| (row + dr, col + dc))
    .filter(|&(r, c)| r >= 0 && c >= 0)
    .map(|(r, c)| parts.at(r as u8, c as u8))
    .find(|&p| p != 0)
    .unwrap_or(0)
}

/// A room with a security door, as [`Graph::first_door`] needs it: its
/// parts with the door shut, the ones its booth and its passage are in, and
/// the rooms its passage leads to.
#[derive(Clone, Debug, PartialEq, Eq)]
struct Door {
    shut: Parts,
    booth: u8,
    passage: u8,
    passage_to: Vec<u16>,
}

/// The planet as the map reads it, for level 5's routes (#52): every place,
/// and the places a step leads to from each. Two rooms join where both have
/// a place for BLOB at the same spot on their shared edge, both ways and in
/// every direction, since level 5 takes it that BLOB can fly anywhere and
/// leaves getting there to the player, except down through a vacuum tube,
/// which only ever carries him up; and where their passages lead to each
/// other. Doors and teleport pads count as open, and the core room is
/// reached from the room to its left. As ZX Sidekick has it
/// (zx-sidekick/zx-sidekick#48, #52).
#[derive(Clone, Debug, Default, PartialEq, Eq)]
pub struct Graph {
    ways: std::collections::BTreeMap<Place, Vec<Place>>,
    booths: Vec<Place>,
    /// Every room's parts with doors open, to find where BLOB is.
    parts: Vec<Parts>,
    /// The rooms with a security door, to tell when a route crosses one.
    doors: std::collections::BTreeMap<u16, Door>,
}

impl Graph {
    /// Builds the graph from every room, read in number order.
    fn new(rooms: &[Room], core_room: u16) -> Graph {
        let mut ways = std::collections::BTreeMap::<Place, Vec<Place>>::new();
        let mut join = |a: Place, b: Place, both: bool| {
            if a.1 == 0 || b.1 == 0 {
                return;
            }
            for (from, to) in [(a, b), (b, a)].into_iter().take(if both { 2 } else { 1 }) {
                let list = ways.entry(from).or_default();
                if !list.contains(&to) {
                    list.push(to);
                }
            }
        };
        for (i, r) in rooms.iter().enumerate() {
            let room = i as u16;
            if room == core_room {
                continue;
            }
            if room % COLS + 1 < COLS
                && let Some(o) = rooms.get(i + 1)
            {
                if room + 1 == core_room {
                    for row in FIRST_ROW..LAST_ROW {
                        join((room, r.open.at(row, LAST_COL - 1)), (core_room, 1), false);
                    }
                } else {
                    for row in FIRST_ROW..LAST_ROW {
                        join(
                            (room, r.open.at(row, LAST_COL - 1)),
                            (room + 1, o.open.at(row, 0)),
                            true,
                        );
                    }
                    if r.passage_right && o.passage_left {
                        join(
                            (room, open_part(r, r.passage_part)),
                            (room + 1, open_part(o, o.passage_part)),
                            true,
                        );
                    }
                }
            }
            if let Some(below) = rooms.get(i + usize::from(COLS))
                && room + COLS != core_room
            {
                let lift = |room: &Room, row: usize, c: u8| room.lift[row] & (0b11 << c) != 0;
                for c in 0..LAST_COL {
                    let (up, down) = (
                        (room + COLS, below.open.at(FIRST_ROW, c)),
                        (room, r.open.at(LAST_ROW - 1, c)),
                    );
                    // Up always; down only where no vacuum tube stands at
                    // the edge.
                    join(up, down, false);
                    if !(lift(r, 16, c) || lift(r, 17, c) || lift(below, 0, c) || lift(below, 1, c))
                    {
                        join(down, up, false);
                    }
                }
            }
        }
        let booths = rooms
            .iter()
            .enumerate()
            .filter(|(_, r)| r.booth_part != 0)
            .map(|(i, r)| (i as u16, open_part(r, r.booth_part)))
            .collect();
        let doors = rooms
            .iter()
            .enumerate()
            .filter(|(_, r)| r.openings.door.is_some())
            .map(|(i, r)| {
                let room = i as u16;
                let mut passage_to = Vec::new();
                if r.passage_right && rooms.get(i + 1).is_some_and(|o| o.passage_left) {
                    passage_to.push(room + 1);
                }
                if r.passage_left
                    && i.checked_sub(1)
                        .and_then(|l| rooms.get(l))
                        .is_some_and(|o| o.passage_right)
                {
                    passage_to.push(room - 1);
                }
                let door = Door {
                    shut: r.shut.clone(),
                    booth: r.booth_part,
                    passage: r.passage_part,
                    passage_to,
                };
                (room, door)
            })
            .collect();
        Graph {
            ways,
            booths,
            parts: rooms.iter().map(|r| r.open.clone()).collect(),
            doors,
        }
    }

    /// The place BLOB is in, standing at (`x`, `y`) in `room`: the part
    /// under his top-left cell, or one beside it while he is between cells;
    /// part 0 when none is found.
    pub fn place(&self, room: u16, x: u8, y: u8) -> Place {
        (room, self.part_at(room, (0xBF - y.min(0xBF)) >> 3, x >> 3))
    }

    /// The part of `room` at the screen cell (`row`, `col`), such as an
    /// item's, or the part beside it when BLOB does not fit there; 0 when
    /// none is found.
    pub fn part_at(&self, room: u16, row: u8, col: u8) -> u8 {
        self.parts
            .get(usize::from(room))
            .map_or(0, |parts| part_near(parts, row, col))
    }

    /// The first room along `route`, walked from BLOB at (`x`, `y`) in
    /// `here`, whose security door the route has to pass: a room with a
    /// door entered in one of its parts with the door shut and left from
    /// another, with no part serving both. A room the route only ends in is
    /// not counted. `None` when the route passes no door.
    pub fn first_door(&self, here: u16, x: u8, y: u8, route: &[Step]) -> Option<u16> {
        let rooms: Vec<u16> = std::iter::once(here)
            .chain(route.iter().map(|s| s.room))
            .collect();
        for (i, &room) in rooms.iter().enumerate() {
            let (Some(door), Some(out)) = (self.doors.get(&room), route.get(i)) else {
                continue;
            };
            let entry = if i == 0 {
                vec![part_near(&door.shut, (0xBF - y.min(0xBF)) >> 3, x >> 3)]
            } else if route[i - 1].teleport {
                vec![door.booth]
            } else {
                self.edge(room, door, rooms[i - 1])
            };
            let exit = if out.teleport {
                vec![door.booth]
            } else {
                self.edge(room, door, out.room)
            };
            let known = |parts: &[u8]| parts.iter().any(|&p| p != 0);
            if known(&entry) && known(&exit) && !entry.iter().any(|p| *p != 0 && exit.contains(p)) {
                return Some(room);
            }
        }
        None
    }

    /// The parts of `room`, its door shut, that BLOB can cross to or from
    /// the neighbouring room `other` by.
    fn edge(&self, room: u16, door: &Door, other: u16) -> Vec<u8> {
        let (Some(mine), Some(theirs)) = (
            self.parts.get(usize::from(room)),
            self.parts.get(usize::from(other)),
        ) else {
            return Vec::new();
        };
        // The cells either side of the shared edge: mine, then theirs.
        let cells: Vec<((u8, u8), (u8, u8))> = if other == room.wrapping_add(1) {
            (FIRST_ROW..LAST_ROW)
                .map(|r| ((r, LAST_COL - 1), (r, 0)))
                .collect()
        } else if room == other.wrapping_add(1) {
            (FIRST_ROW..LAST_ROW)
                .map(|r| ((r, 0), (r, LAST_COL - 1)))
                .collect()
        } else if other == room.wrapping_add(COLS) {
            (0..LAST_COL)
                .map(|c| ((LAST_ROW - 1, c), (FIRST_ROW, c)))
                .collect()
        } else if room == other.wrapping_add(COLS) {
            (0..LAST_COL)
                .map(|c| ((FIRST_ROW, c), (LAST_ROW - 1, c)))
                .collect()
        } else {
            Vec::new()
        };
        let mut parts: Vec<u8> = cells
            .into_iter()
            .filter(|&(m, t)| mine.at(m.0, m.1) != 0 && theirs.at(t.0, t.1) != 0)
            .map(|(m, _)| door.shut.at(m.0, m.1))
            .collect();
        if door.passage_to.contains(&other) {
            parts.push(door.passage);
        }
        parts.sort_unstable();
        parts.dedup();
        parts
    }

    /// The places one step from `place`.
    pub fn ways(&self, place: Place) -> &[Place] {
        self.ways.get(&place).map_or(&[], Vec::as_slice)
    }

    /// The fewest steps from `start` to a room in `targets` or a place in
    /// `places`, a teleporter counting as one step: the graph's ways, and
    /// from a booth in a room in `booths` to another such booth. `start`
    /// with part 0 starts from every place in its room. `None` when no
    /// target can be reached; empty when `start` is at one.
    pub fn route(
        &self,
        start: Place,
        booths: &[u16],
        targets: &RoomSet,
        places: &[Place],
    ) -> Option<Vec<Step>> {
        use std::collections::{BTreeMap, BTreeSet, VecDeque};
        let reached = |p: Place| targets.contains(p.0) || places.contains(&p);
        if reached(start) {
            return Some(vec![]);
        }
        let starts: Vec<Place> = if start.1 == 0 {
            self.ways
                .range((start.0, 0)..=(start.0, u8::MAX))
                .map(|(p, _)| *p)
                .collect()
        } else {
            vec![start]
        };
        let seen_booths: Vec<Place> = self
            .booths
            .iter()
            .copied()
            .filter(|b| booths.contains(&b.0))
            .collect();
        let mut came: BTreeMap<Place, (Place, bool)> = BTreeMap::new();
        let mut queue: VecDeque<Place> = starts.iter().copied().collect();
        let mut seen: BTreeSet<Place> = starts.iter().copied().collect();
        while let Some(here) = queue.pop_front() {
            if reached(here) {
                let mut path = vec![];
                let mut at = here;
                while let Some(&(from, teleport)) = came.get(&at) {
                    path.push(Step {
                        room: at.0,
                        teleport,
                    });
                    at = from;
                }
                path.reverse();
                return Some(path);
            }
            let walks = self.ways(here).iter().map(|&to| (to, false));
            let jumps = seen_booths
                .contains(&here)
                .then(|| {
                    seen_booths
                        .iter()
                        .filter(move |&&b| b != here)
                        .map(|&b| (b, true))
                })
                .into_iter()
                .flatten();
            for (to, teleport) in walks.chain(jumps).collect::<Vec<_>>() {
                if seen.insert(to) {
                    came.insert(to, (here, teleport));
                    queue.push_back(to);
                }
            }
        }
        None
    }
}

impl Game {
    /// The planet as a graph of rooms and their parts, for level 5's routes
    /// (#52), each room built into a copy of the game and read.
    pub fn graph(&self) -> Graph {
        let mut scratch = self.clone();
        let rooms: Vec<Room> = (0..COLS * ROWS).map(|r| scratch.read_room(r)).collect();
        Graph::new(&rooms, crate::cores::CORE_ROOM)
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
        divides: Divides::default(),
        door: None,
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
            passage_right: false,
            passage_left: false,
            lift: [0; 18],
            passage_part: 0,
            booth_part: 0,
        };
        let ported: Vec<u8> = ports(&r, false, false).iter().map(|p| p.shut).collect();
        Divides::find(&r, &ported)
    }

    /// A room read from its text for the graph, `#` solid and `L` a
    /// vacuum tube's cell (free), with no passage, door or booth.
    fn graph_room(lines: &[String]) -> Room {
        let refs: Vec<&str> = lines.iter().map(String::as_str).collect();
        let parts = Parts::find(room(&refs));
        let mut lift = [0u32; 18];
        for (r, line) in lines.iter().enumerate() {
            for (c, ch) in line.chars().enumerate() {
                if ch == 'L' {
                    lift[r] |= 1 << c;
                }
            }
        }
        Room {
            openings: scan(room(&refs)),
            shut: parts.clone(),
            open: parts,
            passage: None,
            solid: solid_of(&refs),
            passage_right: false,
            passage_left: false,
            lift,
            passage_part: 0,
            booth_part: 0,
        }
    }

    /// A room open on every side: floor, walls and ceiling with gaps in
    /// the middle.
    fn open_all_round() -> Vec<String> {
        let mut lines = open_both_sides();
        lines[0].replace_range(14..18, "....");
        lines[17].replace_range(14..18, "....");
        lines
    }

    /// Where (`room`, part 1) is: every room here is one part.
    fn to(room: u16) -> RoomSet {
        let mut t = RoomSet::default();
        t.set(room, true);
        t
    }

    #[test]
    fn neighbouring_rooms_open_to_each_other_join_both_ways() {
        let rooms: Vec<Room> = (0..3).map(|_| graph_room(&open_all_round())).collect();
        let g = Graph::new(&rooms, 999);
        let route = g.route((0, 1), &[], &to(2), &[]).unwrap();
        let rooms_on: Vec<u16> = route.iter().map(|s| s.room).collect();
        assert_eq!(rooms_on, [1, 2]);
        assert_eq!(g.route((2, 1), &[], &to(0), &[]).map(|r| r.len()), Some(2));
    }

    #[test]
    fn a_vacuum_tube_is_taken_up_only() {
        // Rooms 0 and 16, one above the other; a tube at the gap in 16's
        // ceiling.
        let mut rooms: Vec<Room> = (0..17).map(|_| graph_room(&walled())).collect();
        rooms[0] = graph_room(&open_all_round());
        let mut below = open_all_round();
        for line in &mut below[0..2] {
            line.replace_range(14..18, "LLLL");
        }
        rooms[16] = graph_room(&below);
        let g = Graph::new(&rooms, 999);
        assert!(g.route((16, 1), &[], &to(0), &[]).is_some(), "up the tube");
        assert!(g.route((0, 1), &[], &to(16), &[]).is_none(), "not down it");
    }

    #[test]
    fn a_booth_whose_code_is_known_jumps_in_one_step() {
        let mut rooms: Vec<Room> = (0..40).map(|_| graph_room(&walled())).collect();
        for r in [3, 37] {
            let mut room = graph_room(&open_both_sides());
            room.booth_part = 1;
            rooms[r] = room;
        }
        let g = Graph::new(&rooms, 999);
        assert!(
            g.route((3, 1), &[], &to(37), &[]).is_none(),
            "no code: no way"
        );
        let route = g.route((3, 1), &[3, 37], &to(37), &[]).unwrap();
        assert_eq!(
            route,
            [Step {
                room: 37,
                teleport: true
            }]
        );
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
