//! Runs the game without a window: random joystick input, periodic
//! screenshots. For testing the whole program end to end.

use std::path::{Path, PathBuf};
use std::rc::Rc;

use starquake::Game;
use starquake::assets::Assets;
use starquake::controls::Input;
use starquake::host::Host;
use starquake::sound::FRAME_T;

use super::video::{FULL_H, FULL_W, draw};

struct Headless {
    frame: u64,
    limit: u64,
    every: u64,
    dir: PathBuf,
    rng: u64,
    input: Input,
    /// How often each blocking effect fired, and the frames it cost.
    tally: [u64; 64],
    lost: [u64; 64],
}

impl Headless {
    fn random(&mut self) -> u64 {
        self.rng ^= self.rng << 13;
        self.rng ^= self.rng >> 7;
        self.rng ^= self.rng << 17;
        self.rng
    }

    fn screenshot(&self, game: &Game) {
        let mut rgba = vec![0u8; FULL_W * FULL_H * 4];
        draw(&game.display.mem, game.display.border, self.frame, &mut rgba);
        let pixels: Vec<u32> = rgba
            .as_chunks::<4>().0.iter()
            .map(|p| (p[0] as u32) << 16 | (p[1] as u32) << 8 | p[2] as u32)
            .collect();
        let path = self.dir.join(format!("frame{:06}.png", self.frame));
        std::fs::write(&path, zx_core::png::encode(&pixels, FULL_W, FULL_H)).expect("write screenshot");
    }
}

impl Host for Headless {
    fn frame(&mut self, game: &Game) -> (Input, u32) {
        let busy: u32 = game.effects.iter().map(|&id| game.assets.beep(id).1).sum();
        for &id in &game.effects {
            let d = game.assets.beep(id).1;
            self.tally[(id & 0x3F) as usize] += 1;
            self.lost[(id & 0x3F) as usize] += (d / FRAME_T) as u64;
        }
        let frames = 1 + busy / FRAME_T;
        let before = self.frame;
        self.frame += frames as u64;
        if before / self.every != self.frame / self.every {
            self.screenshot(game);
        }
        if self.frame >= self.limit {
            println!("ran {} frames; room {}, lives {}, score {:?}", self.frame, game.room, game.status.lives, game.status.score);
            println!("blocking effects during play (frames lost = picture frozen):");
            let mut total = 0;
            for id in 0..64 {
                if self.tally[id] > 0 {
                    println!("  id {:#04x}: {:5} times, {:6} frames lost ({:.1}s)", id, self.tally[id], self.lost[id], self.lost[id] as f64 / f64::from(starquake::host::FRAMES_PER_SECOND));
                    total += self.lost[id];
                }
            }
            println!("  total {} frames lost of {} ({:.0}%)", total, self.frame, 100.0 * total as f64 / self.frame as f64);
            std::process::exit(0);
        }
        self.input.keys = super::scripted_keys(self.frame);
        if self.frame % 12 < frames as u64 {
            // Mostly walking and jumping about, with some firing.
            self.input.kempston = match self.random() % 8 {
                0 => 0,
                1 => 0x01,
                2 => 0x02,
                3 => 0x09,
                4 => 0x0A,
                5 => 0x08,
                6 => 0x11,
                _ => 0x12,
            };
        }
        (self.input, frames)
    }
}

pub fn run(path: &Path, frames: u64, dir: &Path) -> Result<(), String> {
    std::fs::create_dir_all(dir).map_err(|e| e.to_string())?;
    let (memory, loading_screen) = starquake::assets::read_game(path)?;
    let mut parsed = Assets::from_memory(&memory);
    parsed.loading_screen = loading_screen;
    let assets = Rc::new(parsed);
    let mut game = Game::from_memory(assets, &memory);
    let mut host = Headless {
        frame: 0,
        limit: frames,
        every: (frames / 40).max(1),
        dir: dir.to_path_buf(),
        rng: 0x1234_5678,
        input: Input::default(),
        tally: [0; 64],
        lost: [0; 64],
    };
    game.run(&mut host);
    Ok(())
}
