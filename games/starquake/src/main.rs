//! The playable program: a window, keyboard and sound around the game.
//!
//! The game runs on its own thread as sequential code, pausing at each
//! frame boundary (see `starquake::host`). The main thread runs the window:
//! it draws the most recent frame and feeds keyboard state to the game.
//!
//! Usage: `starquake [TAPE] [--headless FRAMES [SCREENSHOT_DIR]]`
//!         `starquake [TAPE] --bench SECONDS`

// Every Rust program links as a console application, which on Windows means
// a command prompt opens behind the game window. A release build asks for
// the windows subsystem instead so that it does not. Debug builds keep the
// console, since that is where anyone debugging wants the output.
//
// The cost is that `eprintln!` reaches nobody when the program is started
// from a file manager, so anything fatal goes through `fatal` below.
#![cfg_attr(all(windows, not(debug_assertions)), windows_subsystem = "windows")]

mod frontend;

use std::path::PathBuf;

/// Reports a fatal startup problem somewhere it can actually be seen, and
/// gives up.
///
/// On Windows a release build has no console, so `eprintln!` goes nowhere
/// when the program is started by double-clicking it — which is exactly how
/// somebody who has just unpacked the archive will start it, and exactly
/// when they are most likely to have forgotten the tape. A message box is
/// the only place that sentence can land. Everywhere else, stderr is right.
fn fatal(message: &str) -> ! {
    eprintln!("{message}");
    #[cfg(all(windows, not(debug_assertions)))]
    {
        use windows_sys::Win32::UI::WindowsAndMessaging::{MB_ICONERROR, MB_OK, MessageBoxW};
        fn wide(s: &str) -> Vec<u16> {
            s.encode_utf16().chain(std::iter::once(0)).collect()
        }
        let (text, title) = (wide(message), wide("Starquake"));
        // SAFETY: both strings are NUL-terminated and outlive the call, and
        // a null window handle is what MessageBoxW wants for an owner-less
        // box.
        unsafe {
            MessageBoxW(
                std::ptr::null_mut(),
                text.as_ptr(),
                title.as_ptr(),
                MB_OK | MB_ICONERROR,
            );
        }
    }
    std::process::exit(1)
}

/// The default name of the tape, when none is given on the command line.
const TAPE: &str = "starquake.tap";

/// Where to look for the tape, in order, when the player has not said.
///
/// Beside the executable comes first, because that is what somebody who has
/// just unpacked a release archive will have done, and it is the case that
/// used not to work. The working directory comes after, so a development
/// checkout still finds `assets/starquake.tap` exactly as it always did.
fn tape_candidates() -> Vec<PathBuf> {
    let mut out = Vec::new();
    if let Ok(exe) = std::env::current_exe()
        && let Some(dir) = exe.parent()
    {
        out.push(dir.join(TAPE));
        out.push(dir.join("assets").join(TAPE));
    }
    out.push(PathBuf::from(TAPE));
    out.push(PathBuf::from("assets").join(TAPE));
    out
}

/// The first candidate that exists, or a message naming every place tried.
fn find_tape() -> Result<PathBuf, String> {
    let tried = tape_candidates();
    if let Some(found) = tried.iter().find(|p| p.is_file()) {
        return Ok(found.clone());
    }
    let mut msg = String::from(
        "error: no copy of Starquake found.\n\n\
         This program contains no part of the original game and reads its \
         graphics, maps,\ntext and sound from your own copy at startup. Put \
         `starquake.tap` next to the\nprogram, or name it on the command \
         line:\n\n    starquake path/to/starquake.tap\n\nLooked in:\n",
    );
    for p in &tried {
        msg.push_str(&format!("  {}\n", p.display()));
    }
    msg.push_str("\nSee ASSETS.md, or assets/README.md in the repository, for where to get one.");
    Err(msg)
}

fn main() {
    let mut args: Vec<String> = std::env::args().skip(1).collect();
    let headless = args.iter().position(|a| a == "--headless").map(|i| {
        let rest: Vec<String> = args.drain(i..).skip(1).collect();
        let frames = rest.first().and_then(|f| f.parse().ok()).unwrap_or(3000);
        let dir = rest
            .get(1)
            .map_or_else(|| PathBuf::from("screenshots"), PathBuf::from);
        (frames, dir)
    });
    // Drained before the path is read, or `starquake --bench 20` would take
    // "--bench" for the tape to load.
    let bench = args.iter().position(|a| a == "--bench").map(|i| {
        let rest: Vec<String> = args.drain(i..).skip(1).collect();
        rest.first().and_then(|s| s.parse().ok()).unwrap_or(20)
    });
    let path = match args.first() {
        Some(given) => PathBuf::from(given),
        None => match find_tape() {
            Ok(found) => found,
            Err(e) => fatal(&e),
        },
    };
    if let Some(secs) = bench {
        if let Err(e) = frontend::bench(&path, secs) {
            fatal(&format!("error: {e}"));
        }
        return;
    }
    let result = match headless {
        Some((frames, dir)) => frontend::headless::run(&path, frames, &dir),
        None => frontend::run(&path),
    };
    if let Err(e) = result {
        fatal(&format!("error: {e}"));
    }
}
