//! Building a room: drawing its tiles and recording the objects its tiles
//! define.
//!
//! A room is a 4 × 3 grid of big blocks, each 8 × 6 characters, below the
//! 6-row HUD. A big block is 2 × 2 tiles on a 4 × 3 character grid (tiles
//! may overlap their neighbours). Before each tile is drawn, its info byte
//! can change the drawing colour or register an object at the tile's
//! position.

use crate::game::Game;
use crate::rng::Rng;

/// First screen row of the room area.
pub const TOP_ROW: u8 = 6;

/// There are 512 rooms. The original stores the room in 16 bits and lets it
/// run past that when BLOB walks off an edge of the map, so the number itself
/// is left alone; only the lookups below mask it, where the original would
/// read past its own tables. The shipped map has no such opening.
pub const ROOM_MASK: u16 = 0x1FF;

/// Address the original's room table starts at. Room seeding is derived
/// from a room's address in that table, so the rewrite needs the number.
const ROOM_TABLE_ADDR: u16 = 0x7530;

/// Size of the sparkle table. (In the original, the spawn-point list
/// follows it directly.)
pub const SPARKLE_TABLE_LEN: usize = 0x62;
/// Room for five force-field records (and a bit).
pub const FORCE_FIELDS_LEN: usize = 0x2F;

/// The restore list's memory, kept like the original's (3-byte records,
/// ended by a record with a zero high address byte).
pub const RESTORE_START: u16 = 0x5B20;
pub const RESTORE_LEN: usize = 0xE0;

/// A marker left by a tile for the game logic, in pixel coordinates
/// (x from the left, y from the bottom).
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct Marker {
    pub x: u8,
    pub y: u8,
    pub kind: u8,
}

/// Objects registered by the room's tiles. Positions are character
/// cells (col, row).
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct RoomObjects {
    /// Tile type 5 positions, as (col, row) byte pairs at an offset derived
    /// from the position. The offset comes from a rotate, so it can be odd
    /// and entries can overlap; the table is therefore kept as bytes.
    pub sparkle_table: Vec<u8>,
    /// Which sparkle slot is animated next, and the sparkle attribute
    /// (high byte).
    pub sparkle_cursor: u16,
    /// Tile type 7 (force fields), raw 8-byte records: col, row, -, timer,
    /// period, active, -, -. A row of 0 ends the list.
    pub force_fields: Vec<u8>,
    /// Number of force fields while the room is built; afterwards the
    /// field animated next.
    pub force_field_cursor: u8,
    /// Tile type 8.
    pub type8: Vec<(u8, u8)>,
    /// Tile type 9: places where a pickup may appear.
    pub spawn_points: Vec<(u8, u8)>,
    /// Markers in placement order.
    pub markers: Vec<Marker>,
    /// Tile type 11 (teleporter): its position and the index of this room's
    /// entry in the teleporter table.
    pub teleport: Option<((u8, u8), usize)>,
    /// Pixel position of the kind-12 marker, if any.
    pub kind12: Option<(u8, u8)>,
}

impl Default for RoomObjects {
    fn default() -> Self {
        RoomObjects {
            sparkle_table: vec![0; SPARKLE_TABLE_LEN],
            sparkle_cursor: 0x6700,
            force_fields: vec![0; FORCE_FIELDS_LEN],
            force_field_cursor: 0,
            type8: Vec::new(),
            spawn_points: Vec::new(),
            markers: Vec::new(),
            teleport: None,
            kind12: None,
        }
    }
}

impl Game {
    /// Appends a (attribute address, value) record to the restore list. With
    /// the list pointer at 0 (while the panel is drawn) the original writes
    /// into ROM, i.e. nowhere, but the pointer still advances.
    fn record_restore(&mut self, attr_offset: usize, attr: u8) {
        let addr = (0x4000 + attr_offset) as u16;
        let p = self.restore_ptr;
        for (i, v) in [addr as u8, (addr >> 8) as u8, attr].into_iter().enumerate() {
            let a = p.wrapping_add(i as u16);
            // Below the list the original writes into ROM, above it into
            // system variables the rewrite does not have: either way the
            // record is lost.
            if let Some(slot) = a.checked_sub(RESTORE_START).map(|o| o as usize)
                && let Some(byte) = self.restore_mem.get_mut(slot) {
                    *byte = v;
                }
        }
        self.restore_ptr = p.wrapping_add(3);
    }

    /// Removes the restore records of a 2 × 2 graphic at (row, col) whose
    /// cells have attribute `attr`. As in the original, the list end then
    /// moves back by eight records rather than four.
    pub fn remove_restore_block(&mut self, row: u8, col: u8, attr: u8) {
        let target = (0x4000 + crate::display::attr_offset_for(crate::display::cell_offset(row, col))) as u16;
        let at = |a: u16| (a.wrapping_sub(RESTORE_START) as usize).min(RESTORE_LEN - 1);
        let mut hl: u16 = 0x5BBE;
        for _ in 0..0x35 {
            let m = &self.restore_mem;
            if m[at(hl)] == attr && m[at(hl - 1)] == (target >> 8) as u8 && m[at(hl - 2)] == target as u8 {
                let entry = hl - 2;
                for i in 0..(0x5BDF - entry) {
                    self.restore_mem[at(entry + i)] = self.restore_mem[at(entry + 12 + i)];
                }
                self.restore_ptr = self.restore_ptr.wrapping_sub(24);
                let mut p = self.restore_ptr;
                loop {
                    self.restore_mem[at(p)] = 0;
                    p += 1;
                    if p as u8 == 0xFF {
                        break;
                    }
                }
                return;
            }
            hl -= 3;
        }
    }

    /// Draws `tile` with its top-left cell at (`row`, `col`).
    /// Draws one tile of a room.
    ///
    /// # Panics
    ///
    /// If a tile's mask and its list of cells disagree, which would mean the
    /// tile data was read wrongly.
    pub fn draw_tile(&mut self, tile: u8, row: u8, col: u8) {
        let r = self.rng.lo() & 7;
        let alt_ink = if r >= 2 { r } else { (row & 7) | 2 };
        let assets = self.assets.clone();
        let t = &assets.tiles[tile as usize];
        let mut cells = t.cells.iter();
        for (dy, mask) in t.masks.iter().enumerate() {
            for dx in 0..8u8 {
                if mask & (0x80 >> dx) == 0 {
                    continue;
                }
                let cell = cells.next().expect("mask and cell count agree");
                let offset = self.display.put_cell(row + dy as u8, col.wrapping_add(dx), &cell.pixels);
                let attr = match cell.attr & 0x3F {
                    0x36 => (cell.attr & 0xC0) | self.colour,
                    0x00 => (cell.attr & 0xF8) | alt_ink,
                    _ => cell.attr,
                };
                self.display.mem[offset] = attr;
                if attr & 0x40 != 0 && attr & 0x38 != 0x20 {
                    self.record_restore(offset, attr);
                }
            }
        }
    }

    /// XORs one character cell onto the screen and sets its attribute. With
    /// bit 7 of `attr` set, only the brightness (for `0x80`/`0xC0`) or the
    /// colour is changed and nothing is recorded for restoring.
    pub fn xor_cell(&mut self, row: u8, col: u8, pixels: &[u8], attr: u8) {
        let base = crate::display::cell_offset(row, col);
        for (line, &p) in pixels.iter().take(8).enumerate() {
            self.display.mem[base + (line << 8)] ^= p;
        }
        let at = crate::display::attr_offset_for(base);
        if attr & 0x80 != 0 {
            let a = attr & 0x7F;
            self.display.mem[at] = if a == 0 || a == 0x40 {
                (self.display.mem[at] & !0x40) | a
            } else {
                a
            };
        } else {
            self.display.mem[at] = attr;
            if attr & 0x40 != 0 {
                self.record_restore(at, attr);
            }
        }
    }

    /// XORs a 2 × 2 character graphic onto the screen at (`row`, `col`).
    pub fn draw_block2x2(&mut self, graphic: &[u8; 32], row: u8, col: u8, attr: u8) {
        let cells = [(row, col), (row, col + 1), (row + 1, col), (row + 1, col + 1)];
        for (k, (r, c)) in cells.into_iter().enumerate() {
            self.xor_cell(r, c, &graphic[k * 8..k * 8 + 8], attr);
        }
    }

    /// Applies a tile's info byte before it is drawn.
    fn tile_info(&mut self, info: u8, row: u8, col: u8) {
        if info == 0 {
            return;
        }
        let kind = info & 0xF0;
        if kind != 0 && kind < 0x50 {
            self.colour = self.room_colours[(kind >> 4) as usize - 1];
            return;
        }
        let col = col.wrapping_add(info & 3);
        let row = row.wrapping_add((info & 0x0C) >> 2);
        match kind {
            0x50 => {
                let c1 = col.wrapping_sub(1);
                let across = c1.rotate_right(1).wrapping_add(c1);
                let down = 0x17u8.wrapping_sub(row) / 3;
                let offset = across.wrapping_add(down).rotate_left(1) as usize;
                self.objects.sparkle_table[offset] = col;
                self.objects.sparkle_table[offset + 1] = row;
            }
            0x70 => {
                let period = (self.rng.lo() & 0x0C) + 8;
                let at = self.objects.force_field_cursor as usize * 8;
                let rec = &mut self.objects.force_fields[at..at + 6];
                rec[0] = col;
                rec[1] = row;
                rec[3] = period;
                rec[4] = period;
                rec[5] = 0;
                self.objects.force_field_cursor += 1;
            }
            0x80 => self.objects.type8.push((col, row)),
            0x90 => self.objects.spawn_points.push((col, row)),
            _ => {
                if kind == 0xB0 {
                    let room = self.room;
                    let index = self
                        .teleporters
                        .iter()
                        .position(|&(lo, hi)| lo as u16 | ((hi >> 7) as u16) << 8 == room)
                        .expect("teleporter tile in a room without a teleporter");
                    self.objects.teleport = Some(((col, row), index));
                }
                self.add_marker(kind >> 4, row, col);
            }
        }
    }

    /// Registers a marker at a character position.
    pub fn add_marker(&mut self, kind: u8, row: u8, col: u8) {
        let x = col.rotate_left(3);
        let y = 0x18u8.wrapping_sub(row).rotate_left(3).wrapping_sub(1);
        self.objects.markers.push(Marker { x, y, kind });
        if kind == 0x0C {
            self.objects.kind12 = Some((x, y));
        }
    }

    fn draw_big_block(&mut self, block: u8, row: u8, col: u8) {
        let tiles = self.assets.big_blocks[block as usize];
        let slots = [(row + 3, col + 4), (row + 3, col), (row, col + 4), (row, col)];
        for (tile, (r, c)) in tiles.into_iter().zip(slots) {
            self.tile_info(self.assets.tile_info[tile as usize], r, c);
            self.rng.step();
            self.draw_tile(tile, r, c);
        }
    }

    fn choose_room_colours(&mut self) {
        self.room_colours = [0; 4];
        for i in 0..4 {
            let colour = loop {
                self.rng.step();
                // Colours 2, 3, 5, 6 (and 7 for the last), never 4 (green).
                let n = if i < 3 { self.rng.lo() & 3 } else { (self.rng.hi() & 0x3F) % 5 };
                let c = if n >= 2 { n + 1 } else { n } + 2;
                if !self.room_colours.contains(&c) {
                    break c;
                }
            };
            self.room_colours[i] = colour;
        }
    }

    /// Draws the current room and records its objects. (The original then
    /// goes on to place pickups; that is not part of this step.)
    pub fn build_room_tiles(&mut self) {
        self.objects = RoomObjects::default();

        let table_addr = ROOM_TABLE_ADDR.wrapping_add(self.room.wrapping_mul(12));
        let layout = self.assets.rooms[(self.room & ROOM_MASK) as usize];
        self.rng = Rng {
            a: table_addr,
            b: u16::from_le_bytes([layout[0], layout[0] ^ 0x5F]),
            c: table_addr,
            b_countdown: 3,
            c_countdown: 3,
        };
        self.rng.step();
        self.choose_room_colours();

        for (i, &block) in layout.iter().enumerate() {
            let row = TOP_ROW + (i / 4) as u8 * 6;
            let col = (i % 4) as u8 * 8;
            self.draw_big_block(block, row, col);
        }
    }
}
