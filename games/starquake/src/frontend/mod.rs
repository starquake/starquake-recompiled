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
use starquake::host::Host;

/// How long a Spectrum frame lasts: 69888 T-states at 3.5 MHz. Not quite
/// 20ms, and the difference is the game running 0.16% slow or fast.
const FRAME_PERIOD: Duration = Duration::from_nanos(19_968_000);

/// State shared between the game thread and the window.
pub struct Shared {
    /// The most recent frame: display memory, border colour, frame number.
    pub screen: Mutex<(Vec<u8>, u8, u64)>,
    pub input: Mutex<Input>,
    pub quit: AtomicBool,
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
            eprintln!("audio rate {} threshold {} samples", out.rate(), out.rate() as usize / 50 * 3);
        }
    }
}

impl Host for FrontHost {
    fn frame(&mut self, game: &Game) -> (Input, u32) {
        if self.shared.quit.load(Ordering::Relaxed) {
            self.report();
            std::process::exit(0);
        }

        self.frame_start = Some(Instant::now());
        let busy = self.beeper.effects(&game.assets.ram, &game.effects);
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
        let samples = self.beeper.take_samples();
        let mut period = FRAME_PERIOD * frames;
        if let Some(out) = &self.audio {
            out.push(&samples);
            // The card's clock and this one drift apart slowly, and a frame
            // of sound is a touch short of what the card eats in a frame. So
            // lean on the period when the buffer strays outside two to three
            // frames' worth, and run at the exact rate while it is happy:
            // enough buffered to ride out a late wake-up, too little to hear.
            let frame = out.rate() as usize / 50;
            let queued = out.queued();
            if queued < frame * 2 {
                period = period.saturating_sub(Duration::from_micros(500));
            } else if queued > frame * 3 {
                period += Duration::from_micros(500);
            }
        }
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
            input.keys[5] &= !0x01;
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
    let mut parsed = Assets::from_memory(&memory);
    parsed.loading_screen = loading_screen;
    let assets = Rc::new(parsed);
    let mut game = Game::from_memory(assets, &memory);
    let rate = audio.as_ref().map_or(44100, |a| a.rate());
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
    // What the tape showed while it loaded, once, before the game proper.
    game.loading_screen(&mut host);
    // The title screen, a game, and the scores, over and over: the frame
    // counter runs throughout, which is what seeds each new game.
    loop {
        match game.menu(&mut host) {
            starquake::menu::Start::Quit => std::process::exit(0),
            starquake::menu::Start::Play(method) => {
                game.new_game(method);
                game.intro(&mut host);
                game.play(&mut host);
                game.game_over(&mut host);
            }
        }
    }
}

/// Runs the game with its real sound and pacing but no window, driving it
/// with scripted input, and reports how long each frame actually took.
pub fn bench(path: &Path, seconds: u64) -> Result<(), String> {
    unsafe { std::env::set_var("SQ_BENCH", "1") };
    let (memory, loading_screen) = starquake::assets::read_game(path)?;
    let shared = Arc::new(Shared {
        screen: Mutex::new((vec![0; starquake::display::BITMAP_LEN + 768], 0, 0)),
        input: Mutex::new(Input::default()),
        quit: AtomicBool::new(false),
    });
    let (audio, stream) = match audio::Output::start() {
        Ok((out, s)) => (Some(out), Some(s)),
        Err(e) => {
            eprintln!("no sound: {e}");
            (None, None)
        }
    };
    let keys = shared.clone();
    std::thread::spawn(move || {
        let mut n = 0u64;
        loop {
            std::thread::sleep(Duration::from_millis(20));
            n += 1;
            let mut i = keys.input.lock().unwrap();
            i.keys = [0xFF; 8];
            // Past the loading screen, then pick the joystick, then start.
            if (20..40).contains(&n) {
                i.keys[4] = !0x01;
            } else if (60..80).contains(&n) {
                i.keys[3] = !0x01;
            } else if n >= 110 && n % 8 < 4 {
                i.keys[4] = !0x01;
            }
            if n % 12 == 0 {
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
    // Reading checks the file is a supported version.
    let (memory, loading_screen) = starquake::assets::read_game(path)?;

    let shared = Arc::new(Shared {
        screen: Mutex::new((vec![0; starquake::display::BITMAP_LEN + 768], 0, 0)),
        input: Mutex::new(Input::default()),
        quit: AtomicBool::new(false),
    });
    let (audio, stream) = match audio::Output::start() {
        Ok((out, stream)) => (Some(out), Some(stream)),
        Err(e) => {
            eprintln!("no sound: {e}");
            (None, None)
        }
    };
    let game_shared = shared.clone();
    std::thread::Builder::new()
        .name("game".into())
        .spawn(move || game_thread(memory, loading_screen, game_shared, audio))
        .map_err(|e| e.to_string())?;

    let result = video::run(shared);
    drop(stream);
    result
}
