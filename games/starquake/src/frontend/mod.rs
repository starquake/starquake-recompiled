//! Window, input and sound.

mod audio;
mod gamepad;
mod guidance;
pub mod headless;
mod input;
mod overlay;
mod panel;
mod prompt;
pub mod tape;
mod text;
mod video;

use std::path::Path;
use std::rc::Rc;
use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::{Arc, Mutex};
use std::time::{Duration, Instant};

use starquake::Game;
use starquake::assets::Assets;
use starquake::controls::Input;
use starquake::game::Scene;
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
    /// The guidance level, training mode and the picker (#1).
    pub guidance: Mutex<guidance::Guidance>,
    /// Which part of the program the game is in, for the panel.
    pub scene: Mutex<Scene>,
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
    /// The scene last passed on to the window.
    scene: Scene,
    /// "End this game" was chosen and the game has not yet ended.
    abandon: bool,
    /// Whether a pad is starting a game from the title screen (#110).
    starting: bool,
}

/// The Spectrum's 0 key: port 0xEFFE, bit 0.
const ZERO_KEY: (u8, u8) = (0xEF, 0);

impl FrontHost {
    /// Holds the game between frames while the guidance picker is open,
    /// taking the gamepad's side of it: up and down choose a row, left and
    /// right change a setting, A does an action, and B or Select goes back.
    /// No time passes for the game, so its pacing starts again from now.
    /// Returns no input for the frame it resumes on, so the button that
    /// closed the picker is not also a shot in the game.
    fn hold_for_picker(&mut self) -> gamepad::Pad {
        while self.shared.guidance.lock().unwrap().picker_open()
            && !self.shared.quit.load(Ordering::Relaxed)
        {
            std::thread::sleep(Duration::from_millis(20));
            let pad = self.pad.poll();
            let mut guidance = self.shared.guidance.lock().unwrap();
            guidance.set_pad(pad.layout);
            if pad.select || pad.cancel() {
                guidance.back();
            }
            if pad.up {
                guidance.focus_up();
            }
            if pad.down {
                guidance.focus_down();
            }
            if pad.left {
                guidance.change(false);
            }
            if pad.right {
                guidance.change(true);
            }
            if pad.confirm() {
                guidance.enter();
                if guidance.take(guidance::Action::Exit) {
                    self.shared.quit.store(true, Ordering::Relaxed);
                }
            }
        }
        self.next_frame = Instant::now();
        // The button that closed the picker is still down: the game does
        // not see it until it is let go.
        self.pad.hold_back_held();
        gamepad::Pad::default()
    }

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
            ms.len(),
            pct(50),
            pct(90),
            pct(99),
            ms[ms.len() - 1]
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
            med(&w),
            w[w.len() - 1],
            med(&q),
            q[q.len() - 1]
        );
        if let Some(out) = &self.audio {
            eprintln!(
                "audio rate {} threshold {} samples",
                out.rate(),
                out.rate() as usize / FRAMES_PER_SECOND as usize * 3
            );
        }
    }
}

impl Host for FrontHost {
    fn frame(&mut self, game: &Game) -> (Input, u32) {
        if self.shared.quit.load(Ordering::Relaxed) {
            if self.bench {
                // There is no window in the bench, and the game is running on
                // the main thread, so nothing else will end the process: the
                // report is what it was for.
                self.report();
                std::process::exit(0);
            }
            // The window has gone. Returning lets the game run on harmlessly
            // for the moment it takes the event loop to finish; tearing the
            // process down from this thread while the main one is inside
            // pixels.render() is what used to risk a crash on exit.
            return (Input::default(), 1);
        }

        self.frame_start = Some(Instant::now());
        self.shared.guidance.lock().unwrap().set_paused(game.paused);
        if game.scene != self.scene {
            let mut guidance = self.shared.guidance.lock().unwrap();
            if game.scene == Scene::Play {
                guidance.new_game();
            }
            guidance.set_playing(game.scene == Scene::Play);
            drop(guidance);
            self.scene = game.scene;
            *self.shared.scene.lock().unwrap() = game.scene;
        }
        // Every room's openings, for the map: the same every game, so found
        // once, on the first frame. It takes about a millisecond.
        if !self.shared.guidance.lock().unwrap().has_openings() {
            let openings = game.all_openings();
            self.shared.guidance.lock().unwrap().set_openings(openings);
        }
        {
            let mut guidance = self.shared.guidance.lock().unwrap();
            match game.scene {
                Scene::Play => {
                    guidance.set_teleporters(&game.teleporters_seen);
                    guidance.set_room(Some(game.room));
                    guidance.set_unvisited(&game.unvisited_rooms);
                    guidance.set_pieces(&game.missing_piece_rooms());
                }
                Scene::GameOver => guidance.set_room(None),
                // The game-over screens are done: the title screen starts
                // from nothing, though the game keeps its lists until the
                // next one starts.
                Scene::Loading | Scene::Menu => guidance.forget_game(),
            }
        }
        let sound = game.frame_sound();
        let frames = sound.frames;
        self.beeper
            .play(&sound.edges, frames * starquake::sound::FRAME_T);

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
        // A gamepad works in every control method: it presses whatever the
        // chosen method reads, the Kempston port or the method's own keys.
        // Start presses the method's pause key, which is not a fixed key
        // either; on a fresh tape it is Space.
        let mut pad = self.pad.poll();
        {
            let mut guidance = self.shared.guidance.lock().unwrap();
            guidance.set_pad(pad.layout);
            if pad.select && !guidance.picker_open() {
                guidance.open();
            }
        }
        if self.shared.guidance.lock().unwrap().picker_open() {
            pad = self.hold_for_picker();
        }
        let mut input = *self.shared.input.lock().unwrap();
        if self
            .shared
            .guidance
            .lock()
            .unwrap()
            .take(guidance::Action::EndGame)
        {
            self.abandon = true;
        }
        // "End this game" holds A, S, D, F and G, the original's own way to
        // abandon a game. BLOB's control reads them on the play loop's own
        // frames only (`play_work`), so they are held on those frames and on
        // no others: not during a death, a door or a teleporter booth, which
        // reads letters for its code. The request lasts until the game has
        // left play, however long that takes.
        if self.abandon {
            if game.scene != Scene::Play {
                self.abandon = false;
            } else if game.play_work {
                input.keys[1] &= !0x1F;
            }
        }
        // On the title screen, Start or fire on a pad is the 0 key, which
        // starts a game (#110): alone, since the menu reads one key held at
        // a time. What is still held as play begins is kept from the game
        // until it is let go, so it is not a pause or a shot.
        if game.on_title && (pad.start || pad.bits & 0x10 != 0) {
            input.press_key(ZERO_KEY);
            self.starting = true;
        } else {
            if std::mem::take(&mut self.starting) {
                self.pad.hold_back_held();
            }
            // The pad splits up's and down's meanings (#112): the D-pad's up
            // and down board and fly; the button for up picks up, the button
            // for down builds.
            input.pad = pad.meaning();
            // While the game is paused, A or B dismisses the notice as it
            // would any dialog (#89), and does nothing else: its press
            // reaches the game as a move only, so it neither builds nor
            // picks up in the frame play goes on, and it is held back until
            // let go.
            if game.paused && pad.buttons & 0x0C != 0 {
                input.pad.up_moves_only = true;
                input.pad.up_picks_only = false;
                input.pad.down_moves_only = true;
                input.pad.down_builds_only = false;
                self.pad.hold_back_held();
            }
            game.controls.press(&mut input, pad.bits);
            if pad.start {
                game.controls.press_pause(&mut input);
            }
        }

        let now = Instant::now();
        if self.bench {
            self.work
                .push((t_work - self.frame_start.unwrap_or(t_work)).as_micros() as u32);
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
        scene: Scene::Loading,
        abandon: false,
        starting: false,
    };
    // The frame counter runs throughout, which is what seeds each new game.
    game.run(&mut host);
    host.shared.quit.store(true, Ordering::Relaxed);
}

/// The state shared between the game and whatever is showing it.
fn new_shared() -> Arc<Shared> {
    Arc::new(Shared {
        screen: Mutex::new((vec![0; starquake::display::BITMAP_LEN + 768], 0, 0)),
        input: Mutex::new(Input::default()),
        quit: AtomicBool::new(false),
        dead: AtomicBool::new(false),
        guidance: Mutex::new(guidance::Guidance::default()),
        scene: Mutex::new(Scene::Loading),
    })
}

/// The sound card, if there is one. The stream has to be held for as long
/// as the sound should play.
fn open_audio() -> (Option<audio::Output>, Option<cpal::Stream>) {
    match audio::Output::start() {
        Ok((out, stream)) => (Some(out), Some(stream)),
        Err(e) => {
            eprintln!("no sound: {e}");
            (None, None)
        }
    }
}

/// Starts the game on its own thread, with sound, from a checked copy of
/// the game. Returns the sound stream, which the caller holds.
fn launch(
    shared: &Arc<Shared>,
    memory: Vec<u8>,
    loading_screen: Option<Vec<u8>>,
) -> Result<Option<cpal::Stream>, String> {
    let (audio, stream) = open_audio();
    let game_shared = shared.clone();
    std::thread::Builder::new()
        .name("game".into())
        .spawn(move || game_thread(memory, loading_screen, game_shared, audio))
        .map_err(|e| e.to_string())?;
    Ok(stream)
}

/// Runs the game with its real sound and pacing but no window, driving it
/// with scripted input, and reports how long each frame actually took.
pub fn bench(path: &Path, seconds: u64) -> Result<(), String> {
    unsafe { std::env::set_var("SQ_BENCH", "1") };
    // Reading checks the file is a supported version.
    let (memory, loading_screen) = tape::read(path)?;
    let shared = new_shared();
    let (audio, stream) = open_audio();
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

/// Runs the game in a window: from `path`, or, with none, from whatever the
/// player locates on the screen that asks for the tape.
pub fn run(path: Option<&Path>) -> Result<(), String> {
    let shared = new_shared();
    let (prompt, stream) = match path {
        Some(path) => {
            let (memory, loading_screen) = tape::read(path)?;
            (None, launch(&shared, memory, loading_screen)?)
        }
        None => (Some(prompt::Prompt::new()), None),
    };
    let launcher = shared.clone();
    let result = video::run(
        shared,
        prompt,
        Box::new(move |memory, loading_screen| launch(&launcher, memory, loading_screen)),
    );
    drop(stream);
    result
}
