//! The guidance panel beside the game, the picker, and the note of how much
//! help a game had (#1), drawn to the approved mockups.
//!
//! Everything is drawn in logical pixels of the whole window: the picture
//! takes the left `PICTURE_W`, the panel the rest.

use starquake::game::{Scene, SeenTeleporter};
use starquake::map::{COLS, ROWS};

use super::guidance::{Choice, Guidance, LEVELS, Setting};
use super::text::{Canvas, Fonts, Rgb, Span, Weight};

/// The window in logical pixels, and the picture's part of it.
pub const WINDOW_W: f32 = 1368.0;
pub const WINDOW_H: f32 = 768.0;
pub const PICTURE_W: f32 = 960.0;

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

/// What each level adds, for the picker.
const ADDS: [&str; 6] = [
    "The original game, no help.",
    "The codes of the teleporters you have seen.",
    "A map of the rooms you have visited.",
    "Not built yet: the missing core pieces, marked on the map.",
    "Not built yet: an arrow along routes you know.",
    "Not built yet: the arrow routed through the whole map.",
];

pub struct Panel {
    fonts: Fonts,
}

impl Panel {
    pub fn new() -> Panel {
        Panel {
            fonts: Fonts::load(),
        }
    }

    /// Draws the overlay: the panel, and the picker over everything when it
    /// is open.
    pub fn draw(&mut self, canvas: &mut Canvas, guidance: &Guidance, scene: Scene) {
        let left = PICTURE_W + 24.0;
        let width = WINDOW_W - PICTURE_W;
        canvas.round_rect(PICTURE_W, 0.0, width, WINDOW_H, 0.0, PANEL);
        canvas.round_rect(PICTURE_W, 0.0, 1.0, WINDOW_H, 0.0, RULE);

        if scene == Scene::GameOver {
            self.score_note(canvas, left, guidance);
        } else {
            self.spaced(canvas, left, 26.0, "GUIDANCE");
            let level = guidance.level();
            let title = if level == 0 {
                "Off".to_string()
            } else {
                format!("Level {level} \u{b7} {}", LEVELS[level as usize])
            };
            self.fonts.text(
                Some(canvas),
                left,
                44.0,
                Some(width - 48.0),
                1.2,
                &[span(&title, 19.0, Weight::SemiBold, BRIGHT)],
            );
            self.badge(canvas, WINDOW_W - 24.0, 28.0, "Esc");
            let below = if level >= 1 {
                self.teleporters(canvas, left, width - 48.0, guidance.teleporters())
            } else {
                WINDOW_H
            };
            if level >= 2 {
                let explored = format!("explored {} of {} rooms", guidance.explored(), COLS * ROWS);
                self.fonts.text(
                    Some(canvas),
                    left,
                    72.0,
                    None,
                    1.0,
                    &[span(&explored, 12.0, Weight::Regular, LABEL)],
                );
                self.map(canvas, guidance, 96.0, below - 16.0);
            } else {
                let lines = if level == 0 {
                    ["No guidance.", "Press Esc or Select to choose a level."]
                } else {
                    ["The map appears at level 2.", ""]
                };
                for (i, line) in lines.into_iter().enumerate() {
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
        }
    }

    /// Level 2 (#2): the planet between `top` and `bottom`, a room to a
    /// square. Every room is a faint dot; visited rooms join into floor, with
    /// a line along each edge that has no opening, so an opening is a gap in
    /// the wall. The teleporters seen are diamonds and the room BLOB is in
    /// is marked.
    fn map(&mut self, canvas: &mut Canvas, guidance: &Guidance, top: f32, bottom: f32) {
        let (cols, rows) = (f32::from(COLS), f32::from(ROWS));
        // 18 pixels a room as in the mockup, smaller when the teleporter codes
        // take more than one row.
        let pitch = ((bottom - top) / rows).floor().min(18.0);
        let unit = pitch / 18.0;
        let x0 = (PICTURE_W + (WINDOW_W - PICTURE_W - pitch * cols) / 2.0).floor();
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
    }

    /// Level 1 (#50): the codes of the teleporters seen this game, along the
    /// bottom of the panel, in the order they were seen. Returns where the
    /// block starts, for what is drawn above it.
    fn teleporters(
        &mut self,
        canvas: &mut Canvas,
        left: f32,
        width: f32,
        seen: &[SeenTeleporter],
    ) -> f32 {
        let (chip_h, gap) = (26.0, 8.0);
        // Lay the chips out in rows first, so the block can sit on the
        // panel's bottom edge however many rows there are.
        let mut rows: Vec<Vec<(String, f32)>> = vec![Vec::new()];
        let mut used = 0.0;
        for teleporter in seen {
            let text = String::from_utf8_lossy(&teleporter.code).into_owned();
            let w = self
                .fonts
                .measure(&[span(&text, 14.0, Weight::SemiBold, CODE)])
                + 16.0;
            if used + w > width && !rows[rows.len() - 1].is_empty() {
                rows.push(Vec::new());
                used = 0.0;
            }
            used += w + gap;
            rows.last_mut().unwrap().push((text, w));
        }
        let lines = if seen.is_empty() {
            1.0
        } else {
            rows.len() as f32
        };
        let top = WINDOW_H - 24.0 - lines * (chip_h + gap) + gap - 22.0;
        self.spaced(canvas, left, top, "TELEPORTERS SEEN");
        if seen.is_empty() {
            self.fonts.text(
                Some(canvas),
                left,
                top + 24.0,
                Some(width),
                1.0,
                &[span(
                    "None yet: a code shows once you enter its booth.",
                    13.0,
                    Weight::Regular,
                    QUIET,
                )],
            );
            return top;
        }
        let mut y = top + 22.0;
        for row in rows {
            let mut x = left;
            for (text, w) in row {
                canvas.round_rect(x, y, w, chip_h, 4.0, CODE_FILL);
                self.fonts.text(
                    Some(canvas),
                    x + 8.0,
                    y + 5.0,
                    None,
                    1.0,
                    &[span(&text, 14.0, Weight::SemiBold, CODE)],
                );
                x += w + gap;
            }
            y += chip_h + gap;
        }
        top
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
        self.arrows(canvas, rx, rw, top + 69.0, focused, level > 0, level < 5);
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
        let step = (nw - 4.0 * gap) / 5.0;
        for i in 1..=5u8 {
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
    /// controller button that does the same; a key written `(A)` is a
    /// controller's face button, drawn round like one. Arrows are drawn bare:
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

    /// A controller button's letter in a small circle at (`x`, `y`); returns
    /// its width.
    fn pad_button(&mut self, canvas: &mut Canvas, x: f32, y: f32, button: &str) -> f32 {
        let d = 20.0;
        canvas.round_rect(x, y, d, d, d / 2.0, BUTTON_LINE);
        canvas.round_rect(x + 1.0, y + 1.0, d - 2.0, d - 2.0, d / 2.0 - 1.0, DIALOG);
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

    /// A small key name in an outline, right-aligned to `right`.
    fn badge(&mut self, canvas: &mut Canvas, right: f32, y: f32, key: &str) {
        let spans = [span(key, 11.0, Weight::SemiBold, LABEL)];
        let w = self.fonts.measure(&spans) + 14.0;
        let x = right - w;
        canvas.round_rect(x, y - 1.0, w, 19.0, 4.0, BADGE_LINE);
        canvas.round_rect(x + 1.0, y, w - 2.0, 17.0, 3.0, PANEL);
        self.fonts
            .text(Some(canvas), x + 7.0, y + 1.0, None, 1.0, &spans);
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
    use super::*;
    use starquake::map::Openings;
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
        g.set_openings(openings);
        g.set_unvisited(&unvisited);
        g.set_room(Some(row * COLS + col));
        g.set_teleporters(&seen);
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
        let mut record = Guidance::default();
        record.set_level(3);
        record.set_training(true);
        let cases = [
            ("level0", Guidance::default(), Scene::Play),
            ("level2", level2, Scene::Play),
            ("level2-many-codes", crowded, Scene::Play),
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
