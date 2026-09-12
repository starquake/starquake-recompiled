//! Window, input and sound.

mod audio;
mod gamepad;
pub mod headless;
mod input;
mod video;

use std::path::Path;
use std::rc::Rc;
use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::{Arc, Mutex};
use std::time::{Duration, Instant};

use starquake::Game;
use starquake::assets::Assets;
use starquake::controls::Input;
use starquake::host::{FRAMES_PER_SECOND, Host};

/// How long a Spectrum frame lasts, from the clock it is derived from
/// rather than written out.
const FRAME_PERIOD: Duration = Duration::from_nanos(zx_core::timing::FRAME_NANOS);

/// Key presses that carry an unattended run past the loading screen and the
/// menu: any key to leave the picture, `1` to pick the joystick, then `0` to
/// start, which afterwards doubles as the any-key the waiting screens want.
pub fn scripted_keys(frame: u64) -> [u8; 8] {
    let mut keys = [0xFFu8; 8];
    if (20..40).contains(&frame) {
        keys[4] = !0x01; // 0
    } else if (60..80).contains(&frame) {
        keys[3] = !0x01; // 1
    } else if frame >= 110 && frame % 8 < 4 {
        keys[4] = !0x01;
    }
    keys
}

/// State shared between the game thread and the window.
pub struct Shared {
    /// The most recent frame: display memory, border colour, frame number.
    pub screen: Mutex<(Vec<u8>, u8, u64)>,
    pub input: Mutex<Input>,
    /// Set when either side wants to stop: the window was closed, or the
    /// game reached the end of its own loop.
    pub quit: AtomicBool,
    /// Set when the game thread stopped without being asked to, so the
    /// window can report it rather than sitting on a frozen picture.
    pub dead: AtomicBool,
}

/// The game thread's side of the frontend.
struct FrontHost {
    shared: Arc<Shared>,
    audio: Option<audio::Output>,
    beeper: audio::Beeper,
    pad: gamepad::Gamepad,
    next_frame: Instant,
    frame: u64,
    /// Timing, for the bench mode: when the last frame ended, and how long
    /// each took in milliseconds.
    last: Option<Instant>,
    times: Vec<(u32, u32)>,
    work: Vec<u32>,
    wait: Vec<u32>,
    frame_start: Option<Instant>,
    bench: bool,
}

impl FrontHost {
    /// Prints how long frames actually took: the spread, and the worst.
    fn report(&mut self) {
        if self.times.is_empty() {
            return;
        }
        let mut ms: Vec<u32> = self.times.iter().map(|t| t.0).collect();
        ms.sort_unstable();
        let pct = |p: usize| ms[(ms.len() - 1) * p / 100];
        eprintln!(
            "frames {}  median {}ms  p90 {}ms  p99 {}ms  max {}ms",
            ms.len(), pct(50), pct(90), pct(99), ms[ms.len() - 1]
        );
        let mut worst = self.times.clone();
        worst.sort_by_key(|t| std::cmp::Reverse(t.0));
        worst.truncate(12);
        eprintln!("worst frames (ms, frames the game was told passed):");
        for (t, f) in worst {
            eprintln!("  {t:4}ms  frames={f}");
        }
        let stalls = ms.iter().filter(|&&t| t >= 100).count();
        eprintln!("frames over 100ms: {stalls}");
        let mut w = self.work.clone();
        w.sort_unstable();
        let mut q = self.wait.clone();
        q.sort_unstable();
        let med = |v: &Vec<u32>| v[v.len() / 2];
        eprintln!(
            "per frame: work median {}us max {}us | throttle wait median {}us max {}us",
            med(&w), w[w.len() - 1], med(&q), q[q.len() - 1]
        );
        if let Some(out) = &self.audio {
            eprintln!("audio rate {} threshold {} samples", out.rate(), out.rate() as usize / FRAMES_PER_SECOND as usize * 3);
        }
    }
}

impl Host for FrontHost {
    fn frame(&mut self, game: &Game) -> (Input, u32) {
        if self.shared.quit.load(Ordering::Relaxed) {
            self.report();
            // The window has gone. Returning lets the game run on harmlessly
            // for the moment it takes the event loop to finish; tearing the
            // process down from this thread while the main one is inside
            // pixels.render() is what used to risk a crash on exit.
            return (Input::default(), 1);
        }

        self.frame_start = Some(Instant::now());
        let busy = self.beeper.effects(&game.assets, &game.effects);
        let frames = 1 + busy / starquake::sound::FRAME_T;
        let rest = starquake::sound::FRAME_T - busy % starquake::sound::FRAME_T;
        if game.music.is_empty() {
            self.beeper.tone(game.tone, rest);
        } else {
            self.beeper.music(&game.music, rest);
        }

        {
            let mut screen = self.shared.screen.lock().unwrap();
            let n = screen.0.len();
            screen.0.copy_from_slice(&game.display.mem[..n]);
            screen.1 = game.display.border;
            screen.2 = self.frame;
        }
        self.frame += frames as u64;

        let t_work = Instant::now();
        // Pace by the clock, at the Spectrum's own frame rate. Waiting on the
        // sound card's queue to drain instead would tie the frame to when the
        // card happens to ask for samples, which is coarse and bursty enough
        // to cost several milliseconds a frame.
        let mut period = FRAME_PERIOD * frames;
        if let Some(out) = &self.audio {
            out.push(self.beeper.samples());
            // The card's clock and this one drift apart slowly, and a frame
            // of sound is a touch short of what the card eats in a frame. So
            // lean on the period when the buffer strays outside two to three
            // frames' worth, and run at the exact rate while it is happy:
            // enough buffered to ride out a late wake-up, too little to hear.
            let frame = out.rate() as usize / FRAMES_PER_SECOND as usize;
            let queued = out.queued();
            if queued < frame * 2 {
                period = period.saturating_sub(Duration::from_micros(500));
            } else if queued > frame * 3 {
                period += Duration::from_micros(500);
            }
        }
        // Whether or not there is a card to play them on.
        self.beeper.clear_samples();
        self.next_frame += period;
        let now = Instant::now();
        if self.next_frame > now {
            std::thread::sleep(self.next_frame - now);
        } else {
            // Fallen behind (a long sound effect, or the machine is busy):
            // give up the lost time rather than trying to catch it back.
            self.next_frame = now;
        }
        // A gamepad drives the same five bits the Kempston port has; Start
        // presses P, which is where the game's pause key lives.
        let (pad_bits, pad_pause) = self.pad.poll();
        let mut input = *self.shared.input.lock().unwrap();
        input.kempston |= pad_bits;
        if pad_pause {
            // Not a fixed key: the pause key depends on the control method,
            // and Kempston keeps whatever the last keyboard method left,
            // which on a fresh tape is Space rather than P.
            let (port, bit) = game.controls.pause;
            if let Some(row) = (0..8).find(|r| port & (1 << r) == 0) {
                input.keys[row] &= !(1 << bit);
            }
        }

        let now = Instant::now();
        if self.bench {
            self.work.push((t_work - self.frame_start.unwrap_or(t_work)).as_micros() as u32);
            self.wait.push((now - t_work).as_micros() as u32);
            if let Some(last) = self.last {
                self.times.push(((now - last).as_millis() as u32, frames));
            }
        }
        self.last = Some(now);
        (input, frames)
    }
}

fn game_thread(
    memory: Vec<u8>,
    loading_screen: Option<Vec<u8>>,
    shared: Arc<Shared>,
    audio: Option<audio::Output>,
) {
    let watch = shared.clone();
    let played = std::panic::catch_unwind(std::panic::AssertUnwindSafe(move || {
        play_game(&memory, loading_screen, shared, audio);
    }));
    if played.is_err() {
        watch.dead.store(true, Ordering::Relaxed);
    }
    // Either way the game is over, so the window should come down with it.
    watch.quit.store(true, Ordering::Relaxed);
}

fn play_game(
    memory: &[u8],
    loading_screen: Option<Vec<u8>>,
    shared: Arc<Shared>,
    audio: Option<audio::Output>,
) {
    let mut parsed = Assets::from_memory(memory);
    parsed.loading_screen = loading_screen;
    let assets = Rc::new(parsed);
    let mut game = Game::from_memory(assets, memory);
    let rate = audio.as_ref().map_or(44100, audio::Output::rate);
    let mut host = FrontHost {
        shared,
        audio,
        beeper: audio::Beeper::new(rate),
        pad: gamepad::Gamepad::new(),
        next_frame: Instant::now(),
        frame: 0,
        last: None,
        times: Vec::new(),
        work: Vec::new(),
        wait: Vec::new(),
        frame_start: None,
        bench: std::env::var_os("SQ_BENCH").is_some(),
    };
    // The frame counter runs throughout, which is what seeds each new game.
    game.run(&mut host);
    host.shared.quit.store(true, Ordering::Relaxed);
}

/// Runs the game with its real sound and pacing but no window, driving it
/// with scripted input, and reports how long each frame actually took.
/// What both ways of running the game need: the game's own copy of the tape,
/// the state shared with whatever is showing it, and a sound card if there is
/// one. The stream has to be held for as long as the sound should play.
type Started = (Vec<u8>, Option<Vec<u8>>, Arc<Shared>, Option<audio::Output>, Option<cpal::Stream>);

fn start(path: &Path) -> Result<Started, String> {
    // Reading checks the file is a supported version.
    let (memory, loading_screen) = starquake::assets::read_game(path)?;
    let shared = Arc::new(Shared {
        screen: Mutex::new((vec![0; starquake::display::BITMAP_LEN + 768], 0, 0)),
        input: Mutex::new(Input::default()),
        quit: AtomicBool::new(false),
        dead: AtomicBool::new(false),
    });
    let (audio, stream) = match audio::Output::start() {
        Ok((out, stream)) => (Some(out), Some(stream)),
        Err(e) => {
            eprintln!("no sound: {e}");
            (None, None)
        }
    };
    Ok((memory, loading_screen, shared, audio, stream))
}

pub fn bench(path: &Path, seconds: u64) -> Result<(), String> {
    unsafe { std::env::set_var("SQ_BENCH", "1") };
    let (memory, loading_screen, shared, audio, stream) = start(path)?;
    let keys = shared.clone();
    std::thread::spawn(move || {
        let mut n = 0u64;
        loop {
            std::thread::sleep(Duration::from_millis(20));
            n += 1;
            let mut i = keys.input.lock().unwrap();
            i.keys = scripted_keys(n);
            if n.is_multiple_of(12) {
                i.kempston = [0x01, 0x02, 0x09, 0x0A, 0x11][(n as usize / 12) % 5];
            }
        }
    });
    let quit = shared.clone();
    std::thread::spawn(move || {
        std::thread::sleep(Duration::from_secs(seconds));
        quit.quit.store(true, Ordering::Relaxed);
    });
    game_thread(memory, loading_screen, shared, audio);
    drop(stream);
    Ok(())
}

pub fn run(path: &Path) -> Result<(), String> {
    let (memory, loading_screen, shared, audio, stream) = start(path)?;
    let game_shared = shared.clone();
    std::thread::Builder::new()
        .name("game".into())
        .spawn(move || game_thread(memory, loading_screen, game_shared, audio))
        .map_err(|e| e.to_string())?;

    let result = video::run(shared);
    drop(stream);
    result
}
