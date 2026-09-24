//! The guidance panel beside the game, the picker, and the note of how much
//! help a game had (#1), drawn to the approved mockups.
//!
//! Everything is drawn in logical pixels of the whole window: the picture
//! takes the left `PICTURE_W`, the panel the rest.

use starquake::game::Scene;
use starquake::map::{COLS, ROWS};

use super::gamepad::Layout;
use super::guidance::{Choice, Found, Guidance, Heroes, Hole, LEVELS, Setting};
use super::text::{Canvas, Fonts, Rgb, Span, Weight};
use starquake::pickups::Kind;

/// The window in logical pixels, and the picture's part of it.
pub const WINDOW_W: f32 = 1368.0;
pub const WINDOW_H: f32 = 768.0;
pub const PICTURE_W: f32 = 960.0;

/// A game pixel in a door's key code card in the codes' rail (#94), and a
/// card not yet answered by what is carried.
const CARD_PIXEL: f32 = 1.5;
const CODE_DIM: Rgb = [0x3e, 0x6a, 0x66];
/// Where the core's square and the codes' rail start (#91).
const BLOCK_TOP: f32 = 70.0;
/// A game pixel in the core's square, a slot's tile, and the gap between.
const CORE_PIXEL: f32 = 2.0;
const CORE_TILE: f32 = 16.0 * CORE_PIXEL + 6.0;
const CORE_GAP: f32 = 4.0;
/// The core's square: three tiles and the gaps between them.
const CORE_SQUARE: f32 = 3.0 * CORE_TILE + 2.0 * CORE_GAP;
/// A delivered slot, and a slot's tile.
const DELIVERED: Rgb = [0x3a, 0x3f, 0x4b];
const TILE: Rgb = [0x1b, 0x1f, 0x29];
const PANEL: Rgb = [0x0f, 0x11, 0x17];
const RULE: Rgb = [0x22, 0x26, 0x2f];
const LABEL: Rgb = [0x6d, 0x73, 0x85];
const BRIGHT: Rgb = [0xe6, 0xe8, 0xee];
const QUIET: Rgb = [0x5a, 0x60, 0x72];
const SOFT: Rgb = [0xaa, 0xb0, 0xbf];
const BADGE_LINE: Rgb = [0x2b, 0x2f, 0x3a];
const DIM: Rgb = [0x08, 0x09, 0x0c];
const DIALOG: Rgb = [0x10, 0x12, 0x18];
const SELECTED: Rgb = [0x1b, 0x20, 0x30];
const ACCENT: Rgb = [0x8f, 0xb4, 0xff];
const ARROW: Rgb = [0x4a, 0x51, 0x63];
const HINT: Rgb = [0x76, 0x7c, 0x8c];
const HINT_KEY: Rgb = [0xa9, 0xaf, 0xbe];
const SWITCH_ON: Rgb = [0x2f, 0x6f, 0x4f];
const SWITCH_OFF: Rgb = [0x2a, 0x2f, 0x3b];
const ON_TEXT: Rgb = [0xea, 0xff, 0xf2];
const TITLE: Rgb = [0xf2, 0xf3, 0xf7];
const VALUE_DIM: Rgb = [0xc9, 0xcd, 0xd8];
const ACCENT_DIM: Rgb = [0x5e, 0x7f, 0xb8];
const NOTCH: Rgb = [0x26, 0x2b, 0x37];
const LABEL_FOCUSED: Rgb = [0xa9, 0xc5, 0xff];
const BUTTON_LINE: Rgb = [0x3a, 0x3f, 0x4c];
const DANGER: Rgb = [0xe0, 0x67, 0x6f];
const DANGER_FILL: Rgb = [0x2a, 0x16, 0x18];
const DANGER_TITLE: Rgb = [0xf3, 0xc6, 0xca];
const DANGER_TEXT: Rgb = [0xe0, 0xa3, 0xa8];
const TRAINING: Rgb = [0xf5, 0xb8, 0x4b];
const CODE: Rgb = [0x7f, 0xd1, 0xc7];
const CODE_FILL: Rgb = [0x14, 0x25, 0x2a];
const PAUSED: Rgb = [0x5d, 0x63, 0x72];
const FLOOR: Rgb = [0x22, 0x2c, 0x45];
const MAP_DOT: Rgb = [0x17, 0x1a, 0x22];
const WALL: Rgb = [0x9a, 0xaa, 0xd0];
const HERE: Rgb = [0xe8, 0xec, 0xf4];
const PIECE: Rgb = [0xf0, 0x7a, 0xb0];
const PIECE_ROOM: Rgb = [0x15, 0x1a, 0x26];
/// An item on the map by what it does (#93): lilac for what opens a door,
/// yellow for the pad key, white for what a pyramid takes; a core piece
/// still wanted is `PIECE`.
const ITEM_DOOR: Rgb = [0x9b, 0x8a, 0xf0];
const ITEM_PAD: Rgb = [0xf5, 0xd0, 0x4b];
const ITEM_TRADE: Rgb = [0xe6, 0xea, 0xf2];
const ITEM_EDGE: Rgb = [0x00, 0x00, 0x00];
const PIECE_ROOM_LINE: Rgb = [0x6b, 0x75, 0x94];

/// What each level adds, for the picker.
const ADDS: [&str; 7] = [
    "The original game, no help.",
    "The codes you have been shown, and the core's nine slots.",
    "A map of the rooms you have walked through.",
    "Core pieces still needed, in the rooms you have walked through.",
    "And in the rooms you have not.",
    "Not built yet: routes to a piece and to the core.",
    "Not built yet: every code, and the whole planet.",
];

pub struct Panel {
    fonts: Fonts,
    /// The letters the connected pad carries, as of the last draw (#88).
    pad: Layout,
}

impl Panel {
    pub fn new() -> Panel {
        Panel {
            fonts: Fonts::load(),
            pad: Layout::Xbox,
        }
    }

    /// Draws the overlay: the panel, and the picker over everything when it
    /// is open.
    pub fn draw(&mut self, canvas: &mut Canvas, guidance: &Guidance, scene: Scene) {
        self.pad = guidance.pad();
        let left = PICTURE_W + 24.0;
        let width = WINDOW_W - PICTURE_W;
        canvas.round_rect(PICTURE_W, 0.0, width, WINDOW_H, 0.0, PANEL);
        canvas.round_rect(PICTURE_W, 0.0, 1.0, WINDOW_H, 0.0, RULE);

        if scene == Scene::GameOver {
            self.score_note(canvas, left, guidance);
        } else {
            // The level in the corner opposite the label, with its name
            // under it (#91).
            self.spaced(canvas, left, 26.0, "GUIDANCE");
            let level = guidance.level();
            let title = if level == 0 {
                "OFF".to_string()
            } else {
                format!("LEVEL {level}")
            };
            let right = WINDOW_W - 24.0;
            let title_w = self.spaced_width(&title);
            self.spaced_colour(canvas, right - title_w, 26.0, &title, 11.0, BRIGHT);
            if level >= 1 {
                let name = [span(
                    LEVELS[usize::from(level)],
                    12.0,
                    Weight::Regular,
                    SOFT,
                )];
                let w = self.fonts.measure(&name);
                self.fonts
                    .text(Some(canvas), right - w, 42.0, None, 1.0, &name);
            }
            // Everything has one place, the same at every level it shows at
            // (#91): the core's square at the top left, the codes in a rail
            // at the right, the map under the square to the panel's bottom.
            if level >= 1 {
                let rail_w = self.codes_rail(canvas, right, BLOCK_TOP, guidance);
                if !guidance.core().is_empty() {
                    self.spaced(canvas, left, BLOCK_TOP, "CORE");
                    self.core_grid(canvas, guidance.core(), left, BLOCK_TOP + 22.0);
                }
                let foot = BLOCK_TOP + 22.0 + CORE_SQUARE;
                let map_w = right - rail_w - 16.0 - left;
                if level >= 2 {
                    self.map(
                        canvas,
                        guidance,
                        level,
                        left,
                        foot + 16.0,
                        WINDOW_H - 24.0,
                        map_w,
                    );
                } else {
                    let spans = [span(
                        "The map appears at level 2.",
                        13.0,
                        Weight::Regular,
                        QUIET,
                    )];
                    let w = self.fonts.measure(&spans);
                    self.fonts.text(
                        Some(canvas),
                        left + (map_w - w) / 2.0,
                        420.0,
                        None,
                        1.0,
                        &spans,
                    );
                }
            } else {
                for (i, line) in ["No guidance.", "Press Esc or Select to choose a level."]
                    .into_iter()
                    .enumerate()
                {
                    let spans = [span(line, 14.0, Weight::Regular, QUIET)];
                    let w = self.fonts.measure(&spans);
                    self.fonts.text(
                        Some(canvas),
                        PICTURE_W + (width - w) / 2.0,
                        340.0 + i as f32 * 22.4,
                        None,
                        1.0,
                        &spans,
                    );
                }
            }
        }

        if guidance.picker_open() {
            self.picker(canvas, guidance);
        } else if guidance.paused() {
            self.paused(canvas);
        }
    }

    /// The notice while the game is held by its pause key (#89): the picture
    /// dimmed, not the panel, and a card saying how to go on, as the game
    /// itself goes on: at a move or fire once the pause key is let go.
    fn paused(&mut self, canvas: &mut Canvas) {
        canvas.shade(0.0, 0.0, PICTURE_W, WINDOW_H, DIM, 158);
        let (w, h) = (440.0, 160.0);
        let x = (PICTURE_W - w) / 2.0;
        let y = (WINDOW_H - h) / 2.0;
        canvas.round_rect(x, y, w, h, 12.0, BADGE_LINE);
        canvas.round_rect(x + 1.0, y + 1.0, w - 2.0, h - 2.0, 11.0, DIALOG);
        self.fonts.text(
            Some(canvas),
            x + 28.0,
            y + 22.0,
            None,
            1.0,
            &[span("Paused", 24.0, Weight::SemiBold, TITLE)],
        );
        self.fonts.text(
            Some(canvas),
            x + 28.0,
            y + 60.0,
            None,
            1.0,
            &[span("Move or fire to go on.", 16.0, Weight::Regular, SOFT)],
        );
        let foot = y + h - 52.0;
        canvas.round_rect(x + 1.0, foot, w - 2.0, 1.0, 0.0, RULE);
        // The firing button is the left one on every pad; `(X)` draws it
        // as the pad has it printed.
        self.hints(
            canvas,
            x + 28.0,
            foot + 16.0,
            &[
                (&["\u{2190}", "\u{2191}", "\u{2192}", "\u{2193}"], "move"),
                (&["Alt", "/", "(X)"], "fire"),
            ],
        );
    }

    /// Level 2 (#2): the planet between `top` and `bottom`, a room to a
    /// square. Every room is a faint dot; visited rooms join into floor, with
    /// a line along each edge that has no opening, so an opening is a gap in
    /// the wall. The teleporters seen are diamonds and the room BLOB is in
    /// is marked. From level 3 (#91), so is every room you have walked
    /// through holding a core piece still needed; from level 4 every such
    /// room, one not visited outlined so the mark has somewhere to sit. The
    /// map fills `width` from `left`, between `top` and `bottom`.
    #[allow(clippy::too_many_arguments, reason = "the level, and where it goes")]
    fn map(
        &mut self,
        canvas: &mut Canvas,
        guidance: &Guidance,
        level: u8,
        left: f32,
        top: f32,
        bottom: f32,
        width: f32,
    ) {
        let (cols, rows) = (f32::from(COLS), f32::from(ROWS));
        // At most 18 pixels a room, as in the mockup.
        let pitch = ((bottom - top) / rows).min(width / cols).floor().min(18.0);
        let unit = pitch / 18.0;
        let x0 = (left + (width - pitch * cols) / 2.0).floor();
        // Level 3 shows what you could have seen; level 4 what you could not.
        let piece = |room: u16| {
            level >= 3 && guidance.piece(room) && (level >= 4 || guidance.visited(room))
        };
        // Levels 3 and 4 (#93): the items lying out on the planet, those
        // seen in rooms walked through at level 3, all of them at level 4.
        let shown = |f: &&Found| level >= 4 || (level >= 3 && f.seen);
        let items: Vec<&Found> = guidance.items().iter().filter(shown).collect();
        // A piece drawn as itself needs no dot; a room not walked through
        // that holds something drawn is outlined, so the mark has somewhere
        // to sit.
        let itself = |room: u16| items.iter().any(|f| f.room == room && f.piece);
        let holds = |room: u16| items.iter().any(|f| f.room == room);
        let rooms = COLS * ROWS;
        let at = |room: u16| {
            (
                x0 + f32::from(room % COLS) * pitch,
                top + f32::from(room / COLS) * pitch,
            )
        };

        for room in 0..rooms {
            let (x, y) = at(room);
            if guidance.visited(room) {
                canvas.round_rect(x, y, pitch, pitch, 0.0, FLOOR);
            } else if piece(room) || holds(room) {
                let (inset, size) = (2.5 * unit, pitch - 5.0 * unit);
                let (x, y) = (x + inset, y + inset);
                canvas.round_rect(x, y, size, size, 2.0 * unit, PIECE_ROOM);
                let dash = Some(2.5 * unit);
                canvas.outline(x, y, size, size, 2.0 * unit, 1.0, dash, PIECE_ROOM_LINE);
            } else {
                let dot = 8.0 * unit;
                let inset = (pitch - dot) / 2.0;
                canvas.round_rect(x + inset, y + inset, dot, dot, 2.0 * unit, MAP_DOT);
            }
        }
        // Walls after all the floor, so no floor covers them.
        let (line, overhang) = (2.0, 1.0);
        for room in (0..rooms).filter(|&r| guidance.visited(r)) {
            let (x, y) = at(room);
            let open = guidance
                .openings()
                .get(room as usize)
                .copied()
                .unwrap_or_default();
            let long = pitch + 2.0 * overhang;
            if !open.up {
                canvas.round_rect(x - overhang, y - overhang, long, line, 0.0, WALL);
            }
            if !open.down {
                canvas.round_rect(x - overhang, y + pitch - overhang, long, line, 0.0, WALL);
            }
            if !open.left {
                canvas.round_rect(x - overhang, y - overhang, line, long, 0.0, WALL);
            }
            if !open.right {
                canvas.round_rect(x + pitch - overhang, y - overhang, line, long, 0.0, WALL);
            }
            // Walls inside, where they stand in the room (#92): its 32 by 18
            // cells stretched over the square, a door's or a pad's every
            // other cell.
            let d = open.divides;
            let (cw, ch) = (pitch / 32.0, pitch / 18.0);
            for r in 0..18 {
                for c in 0..32 {
                    if !d.wall(r, c) || (d.door(r, c) && (r + c) % 2 == 1) {
                        continue;
                    }
                    canvas.round_rect(
                        x + c as f32 * cw,
                        y + r as f32 * ch,
                        cw.max(1.0),
                        ch.max(1.0),
                        0.0,
                        WALL,
                    );
                }
            }
        }
        for seen in guidance.teleporters() {
            let (x, y) = at(seen.room % rooms);
            let (cx, cy, r) = (x + pitch / 2.0, y + pitch / 2.0, 5.0 * unit);
            canvas.triangle([(cx - r, cy), (cx, cy - r), (cx + r, cy)], CODE);
            canvas.triangle([(cx - r, cy), (cx, cy + r), (cx + r, cy)], CODE);
        }
        if let Some(room) = guidance.room() {
            let (x, y) = at(room);
            let (outer, inner) = (3.0 * unit, 6.0 * unit);
            let size = |inset: f32| pitch - 2.0 * inset;
            canvas.round_rect(
                x + outer,
                y + outer,
                size(outer),
                size(outer),
                2.0 * unit,
                HERE,
            );
            canvas.round_rect(x + inner, y + inner, size(inner), size(inner), unit, FLOOR);
        }
        // Over the room BLOB is in, so a piece there still shows.
        for room in (0..rooms).filter(|&r| piece(r) && !itself(r)) {
            let (x, y) = at(room);
            let r = 4.5 * unit;
            let (cx, cy) = (x + pitch / 2.0, y + pitch / 2.0);
            canvas.round_rect(cx - r, cy - r, 2.0 * r, 2.0 * r, r, PIECE);
        }
        // Each item in the game's own graphic, a game pixel as many whole
        // screen pixels as the room holds with a little air, in the colour
        // of what it does, with a pixel of black around it so it stands off
        // the floor.
        let steps = (pitch * canvas.scale / 16.0).floor().clamp(1.0, 4.0);
        let px = steps / canvas.scale;
        for found in items {
            let (x, y) = at(found.room);
            let colour = if found.piece {
                PIECE
            } else {
                match found.kind {
                    Kind::PadKey => ITEM_PAD,
                    Kind::Trade | Kind::Pack => ITEM_TRADE,
                    Kind::Chip(_) | Kind::AnyChip | Kind::DoorCard => ITEM_DOOR,
                }
            };
            let size = 16.0 * px;
            let (ix, iy) = (x + (pitch - size) / 2.0, y + (pitch - size) / 2.0);
            // Cells top-left, top-right, bottom-left, bottom-right.
            let lit = |row: i32, col: i32| {
                if !(0..16).contains(&row) || !(0..16).contains(&col) {
                    return false;
                }
                let cell = (row / 8 * 2 + col / 8) as usize;
                found.graphic[cell * 8 + (row % 8) as usize] & (0x80 >> (col % 8)) != 0
            };
            // The black first, a pixel outside the graphic's own box so an
            // edge touching it is outlined too, then the graphic over it.
            for edge in [true, false] {
                for row in -1..=16i32 {
                    for col in -1..=16i32 {
                        let here = lit(row, col);
                        let draw = if edge {
                            !here && (-1..=1).any(|dr| (-1..=1).any(|dc| lit(row + dr, col + dc)))
                        } else {
                            here
                        };
                        if draw {
                            let ink = if edge { ITEM_EDGE } else { colour };
                            canvas.round_rect(
                                ix + col as f32 * px,
                                iy + row as f32 * px,
                                px,
                                px,
                                0.0,
                                ink,
                            );
                        }
                    }
                }
            }
        }
        // Each security door whose code is in the rail, numbered where it
        // stands in its room with the number beside its code (#94), drawn
        // last with a dark rim so nothing hides it.
        for (i, door) in guidance.doors().iter().enumerate() {
            let Some((row, col)) = guidance
                .openings()
                .get(usize::from(door.room))
                .and_then(|o| o.door)
            else {
                continue;
            };
            let (x, y) = at(door.room);
            // The middle of the door's tile, four cells by three.
            let cx = x + (f32::from(col) + 2.0) / 32.0 * pitch;
            let cy = y + (f32::from(row) + 1.5) / 18.0 * pitch;
            let r = 6.5;
            canvas.round_rect(cx - r, cy - r, 2.0 * r, 2.0 * r, r, ITEM_EDGE);
            canvas.round_rect(
                cx - r + 1.0,
                cy - r + 1.0,
                2.0 * r - 2.0,
                2.0 * r - 2.0,
                r - 1.0,
                CODE_FILL,
            );
            let label = (i + 1).to_string();
            let spans = [span(&label, 10.0, Weight::SemiBold, CODE)];
            let w = self.fonts.measure(&spans);
            self.fonts
                .text(Some(canvas), cx - w / 2.0, cy - 7.0, None, 1.0, &spans);
        }
    }

    /// Level 1 (#50, #91, #94): the codes shown this game in a rail at the
    /// panel's right: the teleporters' one to a line under TELEPORTS, then
    /// under DOORS each security door's three key code cards in the game's
    /// own graphics, numbered as the map numbers the door, each card dim
    /// until something carried answers it. "None yet" stands under a
    /// heading with nothing. Returns the rail's width.
    fn codes_rail(
        &mut self,
        canvas: &mut Canvas,
        right: f32,
        top: f32,
        guidance: &Guidance,
    ) -> f32 {
        let seen = guidance.teleporters();
        let (chip_h, gap) = (26.0, 8.0);
        let chip_w = self
            .fonts
            .measure(&[span("MMMMM", 14.0, Weight::SemiBold, CODE)])
            + 16.0;
        let card = 16.0 * CARD_PIXEL + 4.0;
        let door_w = 14.0 + 3.0 * card + 2.0 * 3.0;
        let width = chip_w.max(door_w).max(self.spaced_width("TELEPORTS"));
        let heading = self.spaced_width("TELEPORTS");
        self.spaced(canvas, right - heading, top, "TELEPORTS");
        let mut y = top + 22.0;
        if seen.is_empty() {
            let none = [span("None yet", 13.0, Weight::Regular, QUIET)];
            let w = self.fonts.measure(&none);
            self.fonts
                .text(Some(canvas), right - w, y, None, 1.0, &none);
        }
        for teleporter in seen {
            let text = String::from_utf8_lossy(&teleporter.code).into_owned();
            let x = right - chip_w;
            canvas.round_rect(x, y, chip_w, chip_h, 4.0, CODE_FILL);
            let spans = [span(&text, 14.0, Weight::SemiBold, CODE)];
            let w = self.fonts.measure(&spans);
            self.fonts.text(
                Some(canvas),
                x + (chip_w - w) / 2.0,
                y + 5.0,
                None,
                1.0,
                &spans,
            );
            y += chip_h + gap;
        }
        if seen.is_empty() {
            y += 22.0;
        }
        y += 14.0;
        let heading = self.spaced_width("DOORS");
        self.spaced(canvas, right - heading, y, "DOORS");
        y += 22.0;
        if guidance.doors().is_empty() {
            let none = [span("None yet", 13.0, Weight::Regular, QUIET)];
            let w = self.fonts.measure(&none);
            self.fonts
                .text(Some(canvas), right - w, y, None, 1.0, &none);
        }
        for (i, door) in guidance.doors().iter().enumerate() {
            let label = (i + 1).to_string();
            let number = [span(&label, 12.0, Weight::SemiBold, SOFT)];
            self.fonts
                .text(Some(canvas), right - door_w, y + 7.0, None, 1.0, &number);
            for (k, graphic) in door.graphics.iter().enumerate() {
                let x = right - 3.0 * card - 2.0 * 3.0 + k as f32 * (card + 3.0);
                canvas.round_rect(x, y, card, card, 3.0, CODE_FILL);
                let colour = if door.answered[k] { CODE } else { CODE_DIM };
                draw_graphic(canvas, graphic, x + 2.0, y + 2.0, CARD_PIXEL, colour);
            }
            y += card + 6.0;
        }
        width
    }

    /// Level 1 (#91): the core's nine slots as a square of three by three
    /// at the panel's top left, in the order the core holds them, each in
    /// the game's own graphic: white while still wanted, dimmed once
    /// delivered, outlined while its piece is carried.
    fn core_grid(&mut self, canvas: &mut Canvas, core: &[Hole], left: f32, top: f32) {
        let px = CORE_PIXEL;
        for (i, hole) in core.iter().enumerate() {
            let x = left + (i % 3) as f32 * (CORE_TILE + CORE_GAP);
            let y = top + (i / 3) as f32 * (CORE_TILE + CORE_GAP);
            canvas.round_rect(x, y, CORE_TILE, CORE_TILE, 3.0, TILE);
            if hole.carried {
                canvas.outline(x, y, CORE_TILE, CORE_TILE, 3.0, 2.0, None, HERE);
            }
            let colour = if hole.open { HERE } else { DELIVERED };
            let inset = (CORE_TILE - 16.0 * px) / 2.0;
            // Cells top-left, top-right, bottom-left, bottom-right, eight
            // rows of eight each.
            for (cell, rows) in hole.graphic.chunks(8).enumerate() {
                let (cx, cy) = ((cell % 2) as f32 * 8.0, (cell / 2) as f32 * 8.0);
                for (r, bits) in rows.iter().enumerate() {
                    for c in 0..8 {
                        if bits & (0x80 >> c) != 0 {
                            canvas.round_rect(
                                x + inset + (cx + c as f32) * px,
                                y + inset + (cy + r as f32) * px,
                                px,
                                px,
                                0.0,
                                colour,
                            );
                        }
                    }
                }
            }
        }
    }

    /// How wide a spaced label is.
    fn spaced_width(&mut self, label: &str) -> f32 {
        let mut w = 0.0;
        for c in label.chars() {
            w += self.fonts.advance(c, 11.0, Weight::SemiBold) + 11.0 * 0.14;
        }
        w
    }

    fn score_note(&mut self, canvas: &mut Canvas, left: f32, guidance: &Guidance) {
        let record = guidance.record();
        self.spaced(canvas, left, 26.0, "THIS GAME");
        self.fonts.text(
            Some(canvas),
            left,
            300.0,
            None,
            1.0,
            &[span("Played with", 13.0, Weight::Regular, LABEL)],
        );
        let (headline, detail) = if record.highest == 0 {
            ("No guidance".to_string(), None)
        } else {
            (
                format!("Guidance up to level {}", record.highest),
                Some(LEVELS[record.highest as usize]),
            )
        };
        self.fonts.text(
            Some(canvas),
            left,
            322.0,
            None,
            1.0,
            &[span(&headline, 22.0, Weight::SemiBold, BRIGHT)],
        );
        let mut y = 356.0;
        if let Some(detail) = detail {
            self.fonts.text(
                Some(canvas),
                left,
                y,
                None,
                1.0,
                &[span(detail, 14.0, Weight::Regular, SOFT)],
            );
            y += 40.0;
        } else {
            y += 18.0;
        }
        if record.training {
            canvas.round_rect(left, y + 6.0, 8.0, 8.0, 4.0, TRAINING);
            self.fonts.text(
                Some(canvas),
                left + 18.0,
                y,
                None,
                1.0,
                &[span(
                    "Training mode was used",
                    15.0,
                    Weight::Regular,
                    BRIGHT,
                )],
            );
            y += 24.0;
        }
        if let Some(heroes) = guidance.heroes() {
            self.heroes(canvas, left, y + 40.0, &heroes);
        }
    }

    /// Beside the game's CORE OF HEROES screen (#90): each entry's initials
    /// and the guidance its game had, the one the last game put in bright.
    fn heroes(&mut self, canvas: &mut Canvas, left: f32, top: f32, heroes: &Heroes) {
        self.spaced(canvas, left, top, "CORE OF HEROES");
        for (i, (name, level)) in heroes.names.iter().zip(heroes.levels).enumerate() {
            let y = top + 26.0 + i as f32 * 26.0;
            let colour = if heroes.this_game == Some(i) {
                BRIGHT
            } else {
                SOFT
            };
            let name = format!("{}  {}", i + 1, String::from_utf8_lossy(name));
            self.fonts.text(
                Some(canvas),
                left,
                y,
                None,
                1.0,
                &[span(&name, 15.0, Weight::SemiBold, colour)],
            );
            let what = match level {
                None => "from the tape".to_string(),
                Some(0) => "no guidance".to_string(),
                Some(l) => format!("level {l} \u{b7} {}", LEVELS[usize::from(l)]),
            };
            self.fonts.text(
                Some(canvas),
                left + 76.0,
                y + 1.0,
                None,
                1.0,
                &[span(&what, 14.0, Weight::Regular, colour)],
            );
        }
    }

    fn picker(&mut self, canvas: &mut Canvas, guidance: &Guidance) {
        canvas.shade(0.0, 0.0, WINDOW_W, WINDOW_H, DIM, 184);
        // Two settings, then the actions, each taller while it waits for its
        // second press.
        let actions: Vec<Setting> = guidance
            .rows()
            .into_iter()
            .filter(|r| matches!(r, Setting::EndGame | Setting::Exit))
            .collect();
        let action_h = |r: Setting| {
            if guidance.armed() == Some(r) {
                54.0
            } else {
                40.0
            }
        };
        let actions_h: f32 = actions.iter().map(|&r| action_h(r) + 4.0).sum::<f32>() - 4.0;
        let (w, h) = (520.0, 372.0 + 8.0 + actions_h + 12.0 + 52.0);
        let x = (WINDOW_W - w) / 2.0;
        let y = (WINDOW_H - h) / 2.0;
        canvas.round_rect(x, y, w, h, 12.0, BADGE_LINE);
        canvas.round_rect(x + 1.0, y + 1.0, w - 2.0, h - 2.0, 11.0, DIALOG);

        self.fonts.text(
            Some(canvas),
            x + 28.0,
            y + 20.0,
            None,
            1.0,
            &[span("Guidance", 19.0, Weight::SemiBold, BRIGHT)],
        );
        let paused = [span("The game is paused", 12.0, Weight::Regular, LABEL)];
        let pw = self.fonts.measure(&paused);
        self.fonts.text(
            Some(canvas),
            x + w - 28.0 - pw,
            y + 27.0,
            None,
            1.0,
            &paused,
        );

        let focus = guidance.focus();
        let level = guidance.level();
        let training = guidance.training();

        // The guidance level: a number and a name, the notches, and what it adds.
        let (rx, rw) = (x + 12.0, w - 24.0);
        let top = y + 56.0;
        let focused = focus == Setting::Level;
        self.setting_box(canvas, rx, top, rw, 184.0, focused, "GUIDANCE LEVEL");
        self.arrows(
            canvas,
            rx,
            rw,
            top + 69.0,
            focused,
            level > 0,
            usize::from(level) < LEVELS.len() - 1,
        );
        let value = if focused { TITLE } else { VALUE_DIM };
        self.centred_in(
            canvas,
            rx,
            rw,
            top + 30.0,
            &[span(&level.to_string(), 40.0, Weight::SemiBold, value)],
        );
        self.centred_in(
            canvas,
            rx,
            rw,
            top + 80.0,
            &[span(LEVELS[level as usize], 17.0, Weight::SemiBold, value)],
        );
        let (nx, nw, gap) = (rx + 16.0, rw - 32.0, 6.0);
        // A notch a level above 0.
        let notches = (LEVELS.len() - 1) as u8;
        let step = (nw - f32::from(notches - 1) * gap) / f32::from(notches);
        for i in 1..=notches {
            let colour = match (i <= level, focused) {
                (true, true) => ACCENT,
                (true, false) => ACCENT_DIM,
                (false, _) => NOTCH,
            };
            canvas.round_rect(
                nx + f32::from(i - 1) * (step + gap),
                top + 116.0,
                step,
                8.0,
                3.0,
                colour,
            );
        }
        self.fonts.text(
            Some(canvas),
            nx,
            top + 130.0,
            None,
            1.0,
            &[span("less help", 11.0, Weight::Regular, PAUSED)],
        );
        let more = [span("more help", 11.0, Weight::Regular, PAUSED)];
        let mw = self.fonts.measure(&more);
        self.fonts
            .text(Some(canvas), nx + nw - mw, top + 130.0, None, 1.0, &more);
        self.centred_in(
            canvas,
            rx,
            rw,
            top + 152.0,
            &[span(ADDS[level as usize], 13.0, Weight::Regular, HINT_KEY)],
        );

        // Training mode: off or on.
        let top = y + 250.0;
        let focused = focus == Setting::Training;
        self.setting_box(canvas, rx, top, rw, 106.0, focused, "TRAINING MODE");
        self.arrows(canvas, rx, rw, top + 50.0, focused, training, !training);
        let mid = rx + rw / 2.0;
        for (label, chosen, cx, fill, text) in [
            ("Off", !training, mid - 29.0, SWITCH_OFF, BRIGHT),
            ("On", training, mid + 29.0, SWITCH_ON, ON_TEXT),
        ] {
            let spans = [span(
                label,
                15.0,
                Weight::SemiBold,
                if chosen { text } else { PAUSED },
            )];
            let tw = self.fonts.measure(&spans);
            if chosen {
                canvas.round_rect(cx - tw / 2.0 - 14.0, top + 36.0, tw + 28.0, 28.0, 6.0, fill);
            }
            self.fonts
                .text(Some(canvas), cx - tw / 2.0, top + 41.0, None, 1.0, &spans);
        }
        self.centred_in(
            canvas,
            rx,
            rw,
            top + 76.0,
            &[span(
                "Not built yet: energy will stop draining.",
                13.0,
                Weight::Regular,
                HINT_KEY,
            )],
        );

        // The actions: pressed once, a row turns red and asks again.
        let mut ay = y + 380.0;
        canvas.round_rect(x + 1.0, y + 372.0, w - 2.0, 1.0, 0.0, RULE);
        for &row in &actions {
            let rh = action_h(row);
            let (label, again) = match row {
                Setting::EndGame => ("End this game", "Press Enter or A again to end it"),
                _ => ("Exit Starquake", "Press Enter or A again to exit"),
            };
            let armed = guidance.armed() == Some(row);
            let focused = guidance.focus() == row;
            if armed {
                canvas.round_rect(rx, ay, rw, rh, 10.0, DANGER);
                canvas.round_rect(rx + 2.0, ay + 2.0, rw - 4.0, rh - 4.0, 8.0, DANGER_FILL);
            } else if focused {
                canvas.round_rect(rx, ay, rw, rh, 10.0, ACCENT);
                canvas.round_rect(rx + 2.0, ay + 2.0, rw - 4.0, rh - 4.0, 8.0, SELECTED);
            }
            let colour = if armed {
                DANGER_TITLE
            } else if focused {
                TITLE
            } else {
                VALUE_DIM
            };
            self.fonts.text(
                Some(canvas),
                rx + 16.0,
                ay + 11.0,
                None,
                1.0,
                &[span(label, 15.0, Weight::SemiBold, colour)],
            );
            if armed {
                self.fonts.text(
                    Some(canvas),
                    rx + 16.0,
                    ay + 31.0,
                    None,
                    1.0,
                    &[span(again, 12.0, Weight::Regular, DANGER_TEXT)],
                );
            }
            ay += rh + 4.0;
        }

        // What the keys do.
        let foot = y + h - 52.0;
        canvas.round_rect(x + 1.0, foot, w - 2.0, 1.0, 0.0, RULE);
        self.hints(
            canvas,
            x + 28.0,
            foot + 16.0,
            &[
                (&["\u{2191}", "\u{2193}"], "choose"),
                (&["\u{2190}", "\u{2192}"], "change"),
                (&["Enter", "/", "(A)"], "OK"),
                (&["Esc", "/", "(B)"], "close"),
            ],
        );

        if let Some(choice) = guidance.asking() {
            canvas.shade(x, y, w, h, DIM, 150);
            self.noted_with_score(canvas, guidance, choice);
        }
    }

    /// The question over the picker when leaving it would add to the score
    /// note: exactly what changed since it opened, what the score will say,
    /// and buttons named for what they do.
    fn noted_with_score(&mut self, canvas: &mut Canvas, guidance: &Guidance, choice: Choice) {
        let (was_level, was_training) = guidance.opened();
        let (level, training) = (guidance.level(), guidance.training());
        let record = guidance.record();
        let on_off = |on: bool| if on { "on" } else { "off" };

        let mut changes = Vec::new();
        if level != was_level {
            changes.push(format!(
                "Guidance level {was_level} \u{2192} {level}  ({})",
                LEVELS[level as usize]
            ));
        }
        if training != was_training {
            changes.push(format!(
                "Training mode {} \u{2192} {}",
                on_off(was_training),
                on_off(training)
            ));
        }
        let mut shows = Vec::new();
        if level > record.highest {
            shows.push(format!("guidance up to level {level}"));
        }
        if training && !record.training {
            shows.push("that training mode was used".to_string());
        }
        let later = match shows.len() {
            1 if level > record.highest => "turn it down",
            1 => "turn it off",
            _ => "change them back",
        };
        let explanation = format!(
            "This game's score will show {}. That stays, even if you {later} later.",
            shows.join(" and ")
        );
        let (keep, undo) = match (level != was_level, training != was_training) {
            (true, false) => (
                format!("Keep level {level}"),
                format!("Back to level {was_level}"),
            ),
            (false, true) => (
                "Keep training on".to_string(),
                "Turn training off".to_string(),
            ),
            _ => ("Keep both changes".to_string(), "Undo both".to_string()),
        };

        let w = 440.0;
        let x = (WINDOW_W - w) / 2.0;
        let inner = w - 48.0;
        // Measure the wrapped explanation before placing anything.
        let (_, explanation_h) = self.fonts.text(
            None,
            0.0,
            0.0,
            Some(inner),
            1.5,
            &[span(&explanation, 13.0, Weight::Regular, HINT_KEY)],
        );
        let h =
            58.0 + 22.0 * changes.len() as f32 + 10.0 + explanation_h + 20.0 + 40.0 + 20.0 + 44.0;
        let y = (WINDOW_H - h) / 2.0;
        canvas.round_rect(x, y, w, h, 12.0, BUTTON_LINE);
        canvas.round_rect(x + 1.0, y + 1.0, w - 2.0, h - 2.0, 11.0, DIALOG);
        self.fonts.text(
            Some(canvas),
            x + 24.0,
            y + 20.0,
            None,
            1.0,
            &[span(
                "This will show on your score",
                19.0,
                Weight::SemiBold,
                BRIGHT,
            )],
        );
        let mut ly = y + 58.0;
        for change in &changes {
            self.fonts.text(
                Some(canvas),
                x + 24.0,
                ly,
                None,
                1.0,
                &[span(change, 14.0, Weight::SemiBold, TITLE)],
            );
            ly += 22.0;
        }
        ly += 10.0;
        self.fonts.text(
            Some(canvas),
            x + 24.0,
            ly,
            Some(inner),
            1.5,
            &[span(&explanation, 13.0, Weight::Regular, HINT_KEY)],
        );
        let by = ly + explanation_h + 20.0;
        let bw = (inner - 12.0) / 2.0;
        for (i, (label, this)) in [(keep.as_str(), Choice::Use), (undo.as_str(), Choice::Undo)]
            .into_iter()
            .enumerate()
        {
            let bx = x + 24.0 + i as f32 * (bw + 12.0);
            let chosen = choice == this;
            if chosen {
                canvas.round_rect(bx, by, bw, 40.0, 8.0, ACCENT);
            } else {
                canvas.round_rect(bx, by, bw, 40.0, 8.0, BUTTON_LINE);
                canvas.round_rect(bx + 1.0, by + 1.0, bw - 2.0, 38.0, 7.0, DIALOG);
            }
            let spans = [span(
                label,
                14.0,
                Weight::SemiBold,
                if chosen { DIALOG } else { VALUE_DIM },
            )];
            let tw = self.fonts.measure(&spans);
            self.fonts.text(
                Some(canvas),
                bx + (bw - tw) / 2.0,
                by + 12.0,
                None,
                1.0,
                &spans,
            );
        }
        let foot = y + h - 44.0;
        canvas.round_rect(x + 1.0, foot, w - 2.0, 1.0, 0.0, RULE);
        self.hints(
            canvas,
            x + 24.0,
            foot + 12.0,
            &[
                (&["\u{2190}", "\u{2192}"], "choose"),
                (&["Enter", "/", "(A)"], "confirm"),
                (&["Esc", "/", "(B)"], "back"),
            ],
        );
    }

    /// The box of one setting in the picker, outlined when highlighted, and
    /// its label.
    #[allow(
        clippy::too_many_arguments,
        reason = "a box, whether it is highlighted, and its label"
    )]
    fn setting_box(
        &mut self,
        canvas: &mut Canvas,
        x: f32,
        y: f32,
        w: f32,
        h: f32,
        focused: bool,
        label: &str,
    ) {
        if focused {
            canvas.round_rect(x, y, w, h, 10.0, ACCENT);
            canvas.round_rect(x + 2.0, y + 2.0, w - 4.0, h - 4.0, 8.0, SELECTED);
        }
        let colour = if focused { LABEL_FOCUSED } else { LABEL };
        self.spaced_colour(canvas, x + 16.0, y + 14.0, label, 12.0, colour);
    }

    /// The arrows either side of a setting, bright when they would do
    /// something.
    #[allow(
        clippy::too_many_arguments,
        reason = "where they go and which way they work"
    )]
    fn arrows(
        &mut self,
        canvas: &mut Canvas,
        x: f32,
        w: f32,
        cy: f32,
        focused: bool,
        left: bool,
        right: bool,
    ) {
        let colour = |on: bool| match (on, focused) {
            (true, true) => ACCENT,
            (true, false) => ARROW,
            (false, _) => NOTCH,
        };
        let (l, r) = (x + 16.0, x + w - 16.0);
        canvas.triangle(
            [(l, cy), (l + 14.0, cy - 8.0), (l + 14.0, cy + 8.0)],
            colour(left),
        );
        canvas.triangle(
            [(r, cy), (r - 14.0, cy - 8.0), (r - 14.0, cy + 8.0)],
            colour(right),
        );
    }

    fn centred_in(&mut self, canvas: &mut Canvas, x: f32, w: f32, y: f32, spans: &[Span]) {
        let tw = self.fonts.measure(spans);
        self.fonts
            .text(Some(canvas), x + (w - tw) / 2.0, y, None, 1.0, spans);
    }

    fn key_width(&mut self, key: &str) -> f32 {
        self.fonts
            .measure(&[span(key, 12.0, Weight::SemiBold, HINT_KEY)])
            .max(8.0)
            + 12.0
    }

    /// A row of key hints: each group's keys, then what they do. A `/`
    /// between two keys is drawn as text, for a keyboard key and the
    /// controller button that does the same; a key written `(A)` or `(B)`
    /// is the pad's confirming or cancelling button, drawn round and as
    /// the connected pad has it printed (#88). Arrows are drawn bare:
    /// they mean the arrow keys and the D-pad alike, and an outline would
    /// make them read as keys only.
    fn hints(&mut self, canvas: &mut Canvas, mut x: f32, y: f32, groups: &[(&[&str], &str)]) {
        for (keys, what) in groups {
            for key in *keys {
                if *key == "/" {
                    let slash = [span("/", 12.0, Weight::Regular, HINT)];
                    self.fonts.text(Some(canvas), x, y + 2.0, None, 1.0, &slash);
                    x += self.fonts.measure(&slash) + 4.0;
                } else if matches!(*key, "\u{2190}" | "\u{2191}" | "\u{2192}" | "\u{2193}") {
                    let arrow = [span(key, 14.0, Weight::SemiBold, HINT_KEY)];
                    self.fonts.text(Some(canvas), x, y + 1.0, None, 1.0, &arrow);
                    x += self.fonts.measure(&arrow) + 3.0;
                } else if let Some(button) = key.strip_prefix('(').and_then(|k| k.strip_suffix(')'))
                {
                    x += self.pad_button(canvas, x, y, button) + 4.0;
                } else {
                    x += self.key_cap(canvas, x, y, key) + 4.0;
                }
            }
            let spans = [span(what, 12.0, Weight::Regular, HINT)];
            self.fonts
                .text(Some(canvas), x + 2.0, y + 2.0, None, 1.0, &spans);
            x += self.fonts.measure(&spans) + 16.0;
        }
    }

    /// A controller button in a small circle at (`x`, `y`), as the
    /// connected pad has it printed; returns its width. A and B keep their
    /// letters on an Xbox pad and a Nintendo one alike (what changes is the
    /// button under each); a PlayStation pad has the cross and the circle.
    /// `X` is the left button, which fires: X on an Xbox pad, Y on a
    /// Nintendo one, the square on a PlayStation one.
    fn pad_button(&mut self, canvas: &mut Canvas, x: f32, y: f32, button: &str) -> f32 {
        let d = 20.0;
        canvas.round_rect(x, y, d, d, d / 2.0, BUTTON_LINE);
        canvas.round_rect(x + 1.0, y + 1.0, d - 2.0, d - 2.0, d / 2.0 - 1.0, DIALOG);
        if self.pad == Layout::PlayStation && button == "X" {
            let (cx, cy, r) = (x + d / 2.0, y + d / 2.0, d * 0.22);
            canvas.outline(cx - r, cy - r, 2.0 * r, 2.0 * r, 1.0, 1.8, None, HINT_KEY);
            return d;
        }
        let button = if self.pad == Layout::Nintendo && button == "X" {
            "Y"
        } else {
            button
        };
        if self.pad == Layout::PlayStation && matches!(button, "A" | "B") {
            let (cx, cy, r) = (x + d / 2.0, y + d / 2.0, d * 0.24);
            if button == "A" {
                for (dx, dy) in [(1.0, 1.0), (1.0, -1.0)] {
                    stroke(
                        canvas,
                        (cx - r * dx, cy - r * dy),
                        (cx + r * dx, cy + r * dy),
                        1.8,
                        None,
                        HINT_KEY,
                    );
                }
            } else {
                canvas.outline(cx - r, cy - r, 2.0 * r, 2.0 * r, r, 1.8, None, HINT_KEY);
            }
            return d;
        }
        let spans = [span(button, 11.0, Weight::SemiBold, HINT_KEY)];
        let tw = self.fonts.measure(&spans);
        self.fonts
            .text(Some(canvas), x + (d - tw) / 2.0, y + 3.0, None, 1.0, &spans);
        d
    }

    /// A key name in a small outline at (`x`, `y`); returns its width.
    fn key_cap(&mut self, canvas: &mut Canvas, x: f32, y: f32, key: &str) -> f32 {
        let w = self.key_width(key);
        canvas.round_rect(x, y, w, 20.0, 4.0, BUTTON_LINE);
        canvas.round_rect(x + 1.0, y + 1.0, w - 2.0, 18.0, 3.0, DIALOG);
        self.fonts.text(
            Some(canvas),
            x + 6.0,
            y + 2.0,
            None,
            1.0,
            &[span(key, 12.0, Weight::SemiBold, HINT_KEY)],
        );
        w
    }

    /// A small label with its letters spread out.
    fn spaced(&mut self, canvas: &mut Canvas, x: f32, y: f32, text: &str) {
        self.spaced_colour(canvas, x, y, text, 11.0, LABEL);
    }

    #[allow(clippy::too_many_arguments, reason = "where, what, and how it looks")]
    fn spaced_colour(
        &mut self,
        canvas: &mut Canvas,
        mut x: f32,
        y: f32,
        text: &str,
        size: f32,
        colour: Rgb,
    ) {
        let mut buf = [0u8; 4];
        for c in text.chars() {
            let s = span(c.encode_utf8(&mut buf), size, Weight::SemiBold, colour);
            self.fonts
                .text(Some(canvas), x, y, None, 1.0, std::slice::from_ref(&s));
            x += self.fonts.advance(c, size, Weight::SemiBold) + size * 0.14;
        }
    }
}

/// A game graphic, 16 by 16 in cells top-left, top-right, bottom-left and
/// bottom-right, its set pixels `px` square from (`x`, `y`).
fn draw_graphic(canvas: &mut Canvas, graphic: &[u8; 32], x: f32, y: f32, px: f32, colour: Rgb) {
    for (cell, rows) in graphic.chunks(8).enumerate() {
        let (cx, cy) = ((cell % 2) as f32 * 8.0, (cell / 2) as f32 * 8.0);
        for (r, bits) in rows.iter().enumerate() {
            for c in 0..8 {
                if bits & (0x80 >> c) != 0 {
                    canvas.round_rect(
                        x + (cx + c as f32) * px,
                        y + (cy + r as f32) * px,
                        px,
                        px,
                        0.0,
                        colour,
                    );
                }
            }
        }
    }
}

/// A straight line `width` wide from `a` to `b`, with square ends, dashed
/// when `dash` gives the length of a dash and of a gap.
#[allow(
    dead_code,
    reason = "unused once walls are cells (#92); the pad's marks and the routes draw with it"
)]
fn stroke(
    canvas: &mut Canvas,
    a: (f32, f32),
    b: (f32, f32),
    width: f32,
    dash: Option<f32>,
    colour: Rgb,
) {
    let (dx, dy) = (b.0 - a.0, b.1 - a.1);
    let length = dx.hypot(dy);
    if length == 0.0 {
        return;
    }
    let (ux, uy) = (dx / length, dy / length);
    let (nx, ny) = (-uy * width / 2.0, ux * width / 2.0);
    // Half a width past each end, as the edge lines overhang their corners.
    let (from, to) = (-width / 2.0, length + width / 2.0);
    let (on, step) = dash.map_or((to - from, to - from), |d| (d, 2.0 * d));
    let mut s = from;
    while s < to {
        let e = (s + on).min(to);
        let p = |t: f32| (a.0 + ux * t, a.1 + uy * t);
        let (p0, p1) = (p(s), p(e));
        let corners = [
            (p0.0 + nx, p0.1 + ny),
            (p1.0 + nx, p1.1 + ny),
            (p1.0 - nx, p1.1 - ny),
            (p0.0 - nx, p0.1 - ny),
        ];
        canvas.triangle([corners[0], corners[1], corners[2]], colour);
        canvas.triangle([corners[0], corners[2], corners[3]], colour);
        s += step;
    }
}

fn span(text: &str, size: f32, weight: Weight, colour: Rgb) -> Span<'_> {
    Span {
        text,
        size,
        weight,
        colour,
    }
}

#[cfg(test)]
mod render_check {
    use super::super::guidance::DoorCode;
    use super::*;
    use starquake::game::SeenTeleporter;
    use starquake::map::{Divides, Openings};
    use starquake::pickups::RoomSet;

    /// A made-up exploration, like the mockup's: a random walk over the
    /// map, whose steps are its only openings, with `codes` teleporters seen
    /// on the way. The codes are placeholders; the real ones are the
    /// original's text.
    fn explore(g: &mut Guidance, codes: usize) {
        let mut openings = vec![Openings::default(); usize::from(COLS * ROWS)];
        let mut unvisited = RoomSet([0xFF; 64]);
        let mut seen = Vec::new();
        let (mut col, mut row) = (7u16, 20u16);
        let mut rng = 7u32;
        for step in 0..420 {
            let room = row * COLS + col;
            unvisited.set(room, false);
            if step % 60 == 59 && seen.len() < codes {
                let letter = |k: usize| b'A' + ((seen.len() * 5 + k) % 26) as u8;
                seen.push(SeenTeleporter {
                    room,
                    code: [0, 1, 2, 3, 4].map(letter),
                });
            }
            rng ^= rng << 13;
            rng ^= rng >> 17;
            rng ^= rng << 5;
            let (dc, dr) = [(1, 0), (-1, 0), (0, 1), (0, -1), (1, 0), (-1, 0)][rng as usize % 6];
            let (c, r) = (col as i32 + dc, row as i32 + dr);
            if !(0..i32::from(COLS)).contains(&c) || !(0..i32::from(ROWS)).contains(&r) {
                continue;
            }
            let next = r as u16 * COLS + c as u16;
            let (a, b) = (room.min(next) as usize, room.max(next) as usize);
            if dc != 0 {
                openings[a].right = true;
                openings[b].left = true;
            } else {
                openings[a].down = true;
                openings[b].up = true;
            }
            (col, row) = (c as u16, r as u16);
        }
        // Walls inside a few rooms along the walk: a straight one, a
        // diagonal one, a three-way one and a door.
        let visited: Vec<usize> = (0..openings.len())
            .filter(|&r| !unvisited.contains(r as u16))
            .collect();
        // Walls inside a few rooms along the walk: a bar down the middle, a
        // bar across, and a door's bar.
        let down = |door: bool| {
            let mut d = Divides::default();
            for row in 0..18 {
                d.cells[row] = 0b11 << 15;
                if door {
                    d.doors[row] = d.cells[row];
                }
            }
            d
        };
        let across = {
            let mut d = Divides::default();
            d.cells[8] = u32::MAX;
            d.cells[9] = u32::MAX;
            d
        };
        let shapes = [down(false), across, down(true)];
        for (k, room) in visited.iter().step_by(9).enumerate() {
            openings[*room].divides = shapes[k % shapes.len()];
        }
        g.set_openings(openings);
        g.set_unvisited(&unvisited);
        g.set_room(Some(row * COLS + col));
        g.set_teleporters(&seen);
        // A core of made-up shapes, not the game's: six still wanted, one of
        // them carried, and three delivered.
        let core: Vec<Hole> = (0..9u8)
            .map(|i| {
                let mut graphic = [0u8; 32];
                for (k, row) in graphic.iter_mut().enumerate() {
                    *row = match (k % 8, k / 8) {
                        (0 | 7, _) => 0xFF,
                        (_, 0 | 2) => 0x80 | (1 << (i % 7)),
                        _ => 0x01 | (0x80 >> (i % 7)),
                    };
                }
                Hole {
                    graphic,
                    open: i < 6,
                    carried: i == 2,
                }
            })
            .collect();
        g.set_core(&core);
        // Made-up items, not the game's: a diamond for each, one of every
        // kind, in rooms walked through and not.
        let mut diamond = [0u8; 32];
        for (k, row) in diamond.iter_mut().enumerate() {
            let r = (k / 16) * 8 + k % 8;
            let half = if r < 8 { r } else { 15 - r };
            let bits = (0xFFFFu16 >> (8 - half.min(7))) & (0xFFFFu16 << (8 - half.min(7)));
            *row = if (k / 8) % 2 == 0 {
                (bits >> 8) as u8
            } else {
                bits as u8
            };
        }
        let walked: Vec<u16> = (0..COLS * ROWS).filter(|&r| g.visited(r)).collect();
        let far: Vec<u16> = (0..COLS * ROWS)
            .filter(|&r| !g.visited(r))
            .step_by(37)
            .collect();
        let kinds = [Kind::Chip(b'2'), Kind::PadKey, Kind::Trade, Kind::DoorCard];
        let mut items = Vec::new();
        for (i, &room) in walked.iter().step_by(11).take(6).enumerate() {
            items.push(Found {
                room,
                kind: kinds[i % kinds.len()],
                piece: i == 5,
                graphic: diamond,
                seen: true,
            });
        }
        for (i, &room) in far.iter().take(6).enumerate() {
            items.push(Found {
                room,
                kind: kinds[i % kinds.len()],
                piece: i % 3 == 0,
                graphic: diamond,
                seen: false,
            });
        }
        g.set_items(items);
        // Two made-up doors in rooms walked through, their cards numbered
        // shapes, one answered.
        let mut openings = g.openings().to_vec();
        let doors: Vec<DoorCode> = walked
            .iter()
            .skip(4)
            .step_by(17)
            .take(2)
            .enumerate()
            .map(|(i, &room)| {
                openings[usize::from(room)].door = Some((7, 14));
                DoorCode {
                    room,
                    graphics: [diamond; 3],
                    answered: [i == 0, false, i == 0],
                }
            })
            .collect();
        g.set_openings(openings);
        g.set_doors(doors);
    }

    /// Draws the panel in a few states, over a grey stand-in for the
    /// picture, to PNGs in the folder `SQ_PANEL_PNG` names, for comparing
    /// with the mockups without a window. Does nothing when it is not set.
    #[test]
    fn render_to_png() {
        let Some(out) = std::env::var_os("SQ_PANEL_PNG") else {
            return;
        };
        let out = std::path::PathBuf::from(out);
        let mut picker = Guidance::default();
        picker.set_level(3);
        picker.open();
        let mut level2 = Guidance::default();
        level2.set_level(2);
        explore(&mut level2, 3);
        let mut crowded = Guidance::default();
        crowded.set_level(2);
        explore(&mut crowded, 7);
        let mut level3 = Guidance::default();
        level3.set_level(3);
        explore(&mut level3, 3);
        let mut pieces = RoomSet::default();
        for (col, row) in [(12, 13), (2, 24), (6, 8), (13, 29), (10, 4), (9, 21)] {
            pieces.set(row * COLS + col, true);
        }
        // One in the room BLOB is in, to show its dot over the marker.
        pieces.set(level3.room().unwrap(), true);
        level3.set_pieces(&pieces);
        let mut record = Guidance::default();
        record.set_level(3);
        record.set_training(true);
        let cases = [
            ("level0", Guidance::default(), Scene::Play),
            ("level2", level2, Scene::Play),
            ("level2-many-codes", crowded, Scene::Play),
            ("level3", level3.clone(), Scene::Play),
            (
                "level4",
                {
                    let mut g = level3;
                    g.set_level(4);
                    g
                },
                Scene::Play,
            ),
            (
                "level1-codes",
                {
                    let mut g = Guidance::default();
                    g.set_level(1);
                    explore(&mut g, 6);
                    g
                },
                Scene::Play,
            ),
            (
                "level1-none",
                {
                    let mut g = Guidance::default();
                    g.set_level(1);
                    g
                },
                Scene::Play,
            ),
            ("picker", picker.clone(), Scene::Play),
            (
                "picker-playstation",
                {
                    let mut g = picker.clone();
                    g.set_pad(Layout::PlayStation);
                    g
                },
                Scene::Play,
            ),
            (
                "paused",
                {
                    let mut g = Guidance::default();
                    g.set_level(2);
                    explore(&mut g, 3);
                    g.set_paused(true);
                    g
                },
                Scene::Play,
            ),
            (
                "paused-nintendo",
                {
                    let mut g = Guidance::default();
                    g.set_paused(true);
                    g.set_pad(Layout::Nintendo);
                    g
                },
                Scene::Play,
            ),
            (
                "paused-playstation",
                {
                    let mut g = Guidance::default();
                    g.set_paused(true);
                    g.set_pad(Layout::PlayStation);
                    g
                },
                Scene::Play,
            ),
            (
                "picker-training",
                {
                    let mut g = picker.clone();
                    g.focus_down();
                    g.change(true);
                    g.focus_up();
                    g.change(false);
                    g
                },
                Scene::Play,
            ),
            (
                "picker-end-armed",
                {
                    let mut g = picker;
                    g.set_playing(true);
                    g.focus_down();
                    g.focus_down();
                    g.enter();
                    g
                },
                Scene::Play,
            ),
            (
                "picker-noted-with-score",
                {
                    let mut g = Guidance::default();
                    g.set_level(1);
                    g.open();
                    g.change(true);
                    g.change(true);
                    g.focus_down();
                    g.change(true);
                    g.back();
                    g
                },
                Scene::Play,
            ),
            (
                "picker-noted-level-only",
                {
                    let mut g = Guidance::default();
                    g.set_level(1);
                    g.open();
                    g.change(true);
                    g.change(true);
                    g.back();
                    g
                },
                Scene::Play,
            ),
            ("score", record, Scene::GameOver),
            (
                "score-heroes",
                {
                    let mut g = Guidance::default();
                    g.set_level(3);
                    g.set_heroes(Some(Heroes {
                        names: [
                            *b"JAN", *b"SQ ", *b"BOB", *b"AAA", *b"BBB", *b"CCC", *b"DDD", *b"EEE",
                        ],
                        levels: [Some(3), Some(0), Some(5), None, None, None, None, None],
                        this_game: Some(0),
                    }));
                    g
                },
                Scene::GameOver,
            ),
        ];
        let mut panel = Panel::new();
        for (name, guidance, scene) in cases {
            let scale = 2.0;
            let (w, h) = ((WINDOW_W * scale) as usize, (WINDOW_H * scale) as usize);
            let mut pixels = vec![0u8; w * h * 4];
            let mut canvas = Canvas {
                pixels: &mut pixels,
                width: w,
                height: h,
                scale,
            };
            canvas.clear_transparent();
            panel.draw(&mut canvas, &guidance, scene);
            // Composite over the stand-in picture, as the GPU would.
            let rgb: Vec<u32> = pixels
                .as_chunks::<4>()
                .0
                .iter()
                .map(|p| {
                    let a = u32::from(p[3]);
                    let under = 0x30 * (255 - a) / 255;
                    let c = |v: u8| u32::from(v) + under;
                    c(p[0]) << 16 | c(p[1]) << 8 | c(p[2])
                })
                .collect();
            std::fs::write(
                out.join(format!("panel-{name}.png")),
                zx_core::png::encode(&rgb, w, h),
            )
            .unwrap();
        }
    }
}
