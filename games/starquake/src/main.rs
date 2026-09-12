//! The playable program: a window, keyboard and sound around the game.
//!
//! The game runs on its own thread as sequential code, pausing at each
//! frame boundary (see `starquake::host`). The main thread runs the window:
//! it draws the most recent frame and feeds keyboard state to the game.
//!
//! Usage: `starquake [TAPE] [--headless FRAMES [SCREENSHOT_DIR]]`
//!         `starquake [TAPE] --bench SECONDS`

mod frontend;

use std::path::PathBuf;

fn main() {
    let mut args: Vec<String> = std::env::args().skip(1).collect();
    let headless = args.iter().position(|a| a == "--headless").map(|i| {
        let rest: Vec<String> = args.drain(i..).skip(1).collect();
        let frames = rest.first().and_then(|f| f.parse().ok()).unwrap_or(3000);
        let dir = rest.get(1).map_or_else(|| PathBuf::from("screenshots"), PathBuf::from);
        (frames, dir)
    });
    // Drained before the path is read, or `starquake --bench 20` would take
    // "--bench" for the tape to load.
    let bench = args.iter().position(|a| a == "--bench").map(|i| {
        let rest: Vec<String> = args.drain(i..).skip(1).collect();
        rest.first().and_then(|s| s.parse().ok()).unwrap_or(20)
    });
    let path = args
        .first()
        .map(PathBuf::from)
        .unwrap_or_else(|| PathBuf::from("assets/starquake.tap"));
    if let Some(secs) = bench {
        if let Err(e) = frontend::bench(&path, secs) {
            eprintln!("error: {e}");
            std::process::exit(1);
        }
        return;
    }
    let result = match headless {
        Some((frames, dir)) => frontend::headless::run(&path, frames, &dir),
        None => frontend::run(&path),
    };
    if let Err(e) = result {
        eprintln!("error: {e}");
        std::process::exit(1);
    }
}
