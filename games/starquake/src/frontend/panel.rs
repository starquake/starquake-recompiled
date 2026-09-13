//! The guidance panel beside the game, the picker, and the note of how much
//! help a game had (#1), drawn to the approved mockups.
//!
//! Everything is drawn in logical pixels of the whole window: the picture
//! takes the left `PICTURE_W`, the panel the rest.

use starquake::game::Scene;

use super::guidance::{Guidance, LEVELS};
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
const DESCRIPTION: Rgb = [0x8b, 0x90, 0xa0];
const SELECTED: Rgb = [0x1b, 0x20, 0x30];
const SELECTED_LINE: Rgb = [0x33, 0x50, 0x7e];
const ACCENT: Rgb = [0x8f, 0xb4, 0xff];
const RADIO: Rgb = [0x4a, 0x51, 0x63];
const HINT: Rgb = [0x76, 0x7c, 0x8c];
const HINT_KEY: Rgb = [0xa9, 0xaf, 0xbe];
const SWITCH_ON: Rgb = [0x2f, 0x6f, 0x4f];
const SWITCH_OFF: Rgb = [0x2a, 0x2f, 0x3b];
const KNOB_ON: Rgb = [0x8a, 0xe0, 0xac];
const KNOB_OFF: Rgb = [0x72, 0x7a, 0x8c];
const TRAINING: Rgb = [0xf5, 0xb8, 0x4b];
const PAUSED: Rgb = [0x5d, 0x63, 0x72];

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
            self.badge(canvas, WINDOW_W - 24.0, 28.0, "F1");
            let (first, second) = if level == 0 {
                ("No guidance.", "Press F1 or Select to choose a level.")
            } else {
                (
                    "This level is not built yet.",
                    "It will show here when it is.",
                )
            };
            for (i, line) in [first, second].into_iter().enumerate() {
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

        if guidance.picker_open() {
            self.picker(canvas, guidance);
        }
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
        let (w, h) = (440.0, 420.0);
        let x = (WINDOW_W - w) / 2.0;
        let y = (WINDOW_H - h) / 2.0;
        canvas.round_rect(x, y, w, h, 10.0, BADGE_LINE);
        canvas.round_rect(x + 1.0, y + 1.0, w - 2.0, h - 2.0, 9.0, DIALOG);

        let inner = x + 22.0;
        self.fonts.text(
            Some(canvas),
            inner,
            y + 18.0,
            None,
            1.0,
            &[span("Guidance", 19.0, Weight::SemiBold, BRIGHT)],
        );
        self.fonts.text(
            Some(canvas),
            inner,
            y + 46.0,
            Some(w - 44.0),
            1.45,
            &[span(
                "Each level adds to the ones before it. The highest level you use is noted with your score.",
                13.0,
                Weight::Regular,
                DESCRIPTION,
            )],
        );
        let mut row_y = y + 100.0;
        canvas.round_rect(x + 1.0, row_y, w - 2.0, 1.0, 0.0, RULE);
        row_y += 8.0;
        for (i, name) in LEVELS.iter().enumerate() {
            let selected = i as u8 == guidance.level();
            let (rx, rw, rh) = (x + 8.0, w - 16.0, 36.0);
            if selected {
                canvas.round_rect(rx, row_y, rw, rh, 7.0, SELECTED_LINE);
                canvas.round_rect(rx + 1.0, row_y + 1.0, rw - 2.0, rh - 2.0, 6.0, SELECTED);
            }
            let cx = rx + 14.0;
            let cy = row_y + 10.0;
            canvas.round_rect(
                cx,
                cy,
                16.0,
                16.0,
                8.0,
                if selected { ACCENT } else { RADIO },
            );
            canvas.round_rect(
                cx + 2.0,
                cy + 2.0,
                12.0,
                12.0,
                6.0,
                if selected { SELECTED } else { DIALOG },
            );
            if selected {
                canvas.round_rect(cx + 4.0, cy + 4.0, 8.0, 8.0, 4.0, ACCENT);
            }
            let number = i.to_string();
            self.fonts.text(
                Some(canvas),
                cx + 28.0,
                row_y + 9.0,
                None,
                1.0,
                &[span(&number, 14.0, Weight::SemiBold, BRIGHT)],
            );
            let colour = if selected { BRIGHT } else { SOFT };
            self.fonts.text(
                Some(canvas),
                cx + 50.0,
                row_y + 9.0,
                None,
                1.0,
                &[span(name, 14.0, Weight::Regular, colour)],
            );
            row_y += rh;
        }
        row_y += 8.0;
        canvas.round_rect(x + 1.0, row_y, w - 2.0, 1.0, 0.0, RULE);

        let training = guidance.training();
        self.fonts.text(
            Some(canvas),
            inner,
            row_y + 12.0,
            None,
            1.0,
            &[span("Training mode", 14.0, Weight::SemiBold, BRIGHT)],
        );
        self.fonts.text(
            Some(canvas),
            inner,
            row_y + 32.0,
            None,
            1.0,
            &[span(
                "Not built yet: energy will stop draining.",
                12.0,
                Weight::Regular,
                DESCRIPTION,
            )],
        );
        let sx = x + w - 22.0 - 40.0;
        canvas.round_rect(
            sx,
            row_y + 17.0,
            40.0,
            22.0,
            11.0,
            if training { SWITCH_ON } else { SWITCH_OFF },
        );
        let knob = if training { sx + 21.0 } else { sx + 3.0 };
        canvas.round_rect(
            knob,
            row_y + 20.0,
            16.0,
            16.0,
            8.0,
            if training { KNOB_ON } else { KNOB_OFF },
        );
        row_y += 56.0;
        canvas.round_rect(x + 1.0, row_y, w - 2.0, 1.0, 0.0, RULE);

        let mut hx = inner;
        for (key, what) in [
            ("\u{2191}\u{2193}", " level"),
            ("T", " training"),
            ("F1", " or "),
            ("Select", " close"),
        ] {
            let spans = [
                span(key, 12.0, Weight::SemiBold, HINT_KEY),
                span(what, 12.0, Weight::Regular, HINT),
            ];
            self.fonts
                .text(Some(canvas), hx, row_y + 13.0, None, 1.0, &spans);
            hx += self.fonts.measure(&spans) + if what == " or " { 4.0 } else { 18.0 };
        }
        self.fonts.text(
            Some(canvas),
            24.0,
            WINDOW_H - 30.0,
            None,
            1.0,
            &[span(
                "The game is paused while this is open.",
                12.0,
                Weight::Regular,
                PAUSED,
            )],
        );
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
    fn spaced(&mut self, canvas: &mut Canvas, mut x: f32, y: f32, text: &str) {
        let mut buf = [0u8; 4];
        for c in text.chars() {
            let s = span(c.encode_utf8(&mut buf), 11.0, Weight::SemiBold, LABEL);
            self.fonts
                .text(Some(canvas), x, y, None, 1.0, std::slice::from_ref(&s));
            x += self.fonts.advance(c, 11.0, Weight::SemiBold) + 1.5;
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
        picker.toggle_picker();
        let mut level2 = Guidance::default();
        level2.set_level(2);
        let mut record = Guidance::default();
        record.set_level(3);
        record.toggle_training();
        let cases = [
            ("level0", Guidance::default(), Scene::Play),
            ("level2", level2, Scene::Play),
            ("picker", picker, Scene::Play),
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
