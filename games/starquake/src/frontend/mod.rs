//! Window, input and sound.

mod audio;
mod gamepad;
mod guidance;
pub mod headless;
mod input;
mod overlay;
mod panel;
mod prompt;
mod scores;
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
    /// The scene the last frame's input was built in, to tell the first
    /// frame of play (#114).
    scene_seen: Scene,
    /// The high-score table kept between runs (#90), and where.
    keeper: scores::Keeper,
    scores_path: Option<std::path::PathBuf>,
}

/// "End this game" (#125): holds A, S, D, F and G, the original's own way
/// to abandon a game, and says whether the request still stands. BLOB's
/// control reads them on the play loop's own frames only (`play_work`), so
/// they are held on those frames and on no others: not during a death, a
/// door or a teleporter booth, which reads letters for its code. A paused
/// game waits for a move before it reads them, so while it is paused a move
/// is given with them: play goes on for the one frame in which BLOB's
/// control sees the keys and ends the game, before anything moves. The
/// request lasts until the game has left play, however long that takes.
fn end_game(scene: Scene, play_work: bool, paused: bool, input: &mut Input) -> bool {
    if scene != Scene::Play {
        return false;
    }
    if play_work || paused {
        input.keys[1] &= !0x1F;
    }
    if paused {
        input.pad.bits |= 0x01;
    }
    true
}

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

/// The core's nine slots as the game has them (#91): each slot's piece, in
/// its graphic, still wanted while its top bit is set (a delivered slot
/// holds its own number), and carried while it is wanted and that graphic is
/// in the inventory.
fn core_holes(game: &Game) -> Vec<guidance::Hole> {
    game.core_slots
        .iter()
        .map(|&slot| {
            let graphic = slot & 0x7F;
            let open = slot >= 0x80;
            guidance::Hole {
                graphic: game.assets.graphic32(graphic),
                open,
                carried: open && game.status.inventory.iter().any(|&(g, _)| g == graphic),
            }
        })
        .collect()
}

/// The items lying out on the planet (#93). A row of 1 to 5 is carried,
/// and the core's own room is not a place an item lies; the packs act as
/// they are picked up, so they are nothing to go and fetch. Everything else
/// is out there whether or not its room has been walked through: the game
/// knows where each one is from the start, which is what level 4 tells and
/// level 3 does not. An item not yet put in its room has row 0.
fn found(game: &Game) -> Vec<guidance::Found> {
    use starquake::pickups::{Kind, kind};
    game.items
        .iter()
        .filter(|item| {
            item.room() != starquake::cores::CORE_ROOM
                && !(1..=5).contains(&item.row())
                && kind(item.graphic()) != Kind::Pack
        })
        .map(|item| guidance::Found {
            room: item.room(),
            kind: kind(item.graphic()),
            piece: game
                .core_slots
                .iter()
                .any(|&slot| slot & 0x80 != 0 && slot & 0x7F == item.graphic()),
            graphic: game.assets.graphic32(item.graphic()),
            seen: item.row() != 0 && !game.unvisited_rooms.contains(item.room()),
        })
        .collect()
}

/// The security doors whose codes the game has shown (#94), each card
/// lit when what is carried answers it, as the door's screen checks them.
fn door_codes(game: &Game) -> Vec<guidance::DoorCode> {
    let slots = game.status.inventory.map(|(g, _)| g);
    game.doors_seen
        .iter()
        .map(|door| {
            let answered = starquake::screens::answered(&door.cards, slots);
            guidance::DoorCode {
                room: door.room,
                graphics: door.cards.map(|c| game.assets.graphic32(c)),
                answered: std::array::from_fn(|i| answered[i]),
            }
        })
        .collect()
}

impl Host for FrontHost {
    fn heroes(&mut self, table: &[u8], new: Option<usize>) {
        let mut guidance = self.shared.guidance.lock().unwrap();
        if let Some(kept) = self.keeper.heroes(table, new, guidance.record())
            && let Some(path) = &self.scores_path
            && let Err(e) = scores::save(path, kept)
        {
            eprintln!("the high scores were not kept: {e}");
        }
        let kept = &self.keeper.kept;
        guidance.set_heroes(Some(guidance::Heroes {
            names: std::array::from_fn(|i| kept.name(i)),
            levels: kept.levels,
            this_game: self.keeper.this_game,
        }));
    }

    fn heroes_shown(&mut self) -> Option<Vec<u8>> {
        self.keeper.shown()
    }

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
            if game.scene != Scene::GameOver {
                guidance.set_heroes(None);
            }
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
                    guidance.set_core(&core_holes(game));
                    guidance.set_items(found(game));
                    guidance.set_doors(door_codes(game));
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
        // A gamepad is its own input (#123): it moves and fires in every
        // control method and Start pauses, but it presses no keys, so it
        // never types into what the game reads as letters.
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
        // On the title screen, Start or fire on a pad starts a game (#110).
        // What is still held as play begins is kept from the game until it
        // is let go, so it is not a pause or a shot.
        if game.on_title && (pad.start || pad.bits & 0x10 != 0) {
            input.pad.start = true;
            self.starting = true;
        } else {
            // The frame after a start, and the first frame of play: what
            // the pad holds is dropped now, and held back from the next
            // poll until it is let go (#114). Whatever took the game past the
            // title and its intro (Start, A, B or fire) does not also pause
            // it, build a platform, pick up or fire as play begins.
            let entering_play = game.scene == Scene::Play && self.scene_seen != Scene::Play;
            self.scene_seen = game.scene;
            if std::mem::take(&mut self.starting) || entering_play {
                self.pad.hold_back_held();
                pad = gamepad::Pad {
                    layout: pad.layout,
                    ..gamepad::Pad::default()
                };
            }
            // The pad splits up's and down's meanings (#112): the D-pad's up
            // and down board and fly; the button for up picks up, the button
            // for down builds.
            input.pad = starquake::controls::PadInput {
                bits: pad.bits,
                start: pad.start,
                meaning: pad.meaning(),
            };
            // While the game is paused, A or B dismisses the notice as it
            // would any dialog (#89), and does nothing else: its press
            // reaches the game as a move only, so it neither builds nor
            // picks up in the frame play goes on, and it is held back until
            // let go.
            if game.paused && pad.buttons & 0x0C != 0 {
                input.pad.meaning = starquake::controls::PadMeaning {
                    up_moves_only: true,
                    up_picks_only: false,
                    down_moves_only: true,
                    down_builds_only: false,
                };
                self.pad.hold_back_held();
            }
        }

        if self.abandon {
            self.abandon = end_game(game.scene, game.play_work, game.paused, &mut input);
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
    // The table kept from earlier runs, in place of the tape's own (#90).
    let scores_path = scores::path();
    let file = scores_path
        .as_ref()
        .and_then(|p| std::fs::read_to_string(p).ok());
    let keeper = scores::Keeper::new(file.as_deref(), &game.high_scores);
    game.high_scores.clone_from(&keeper.kept.table);
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
        scene_seen: Scene::Loading,
        keeper,
        scores_path,
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

#[cfg(test)]
mod tests {
    use super::*;

    /// A S D F G, all held: what BLOB's control ends a game on.
    fn ending(input: &Input) -> bool {
        input.keyboard(0xFD) & 0x1F == 0
    }

    #[test]
    fn ending_a_game_holds_the_keys_on_frames_of_play_only() {
        let mut input = Input::default();
        assert!(end_game(Scene::Play, true, false, &mut input));
        assert!(ending(&input));
        assert_eq!(input.pad.bits, 0, "no move while playing");

        let mut input = Input::default();
        assert!(
            end_game(Scene::Play, false, false, &mut input),
            "still asked"
        );
        assert!(!ending(&input), "not during a death, a door or a booth");
    }

    #[test]
    fn ending_a_paused_game_gives_the_move_that_resumes_it() {
        let mut input = Input::default();
        assert!(end_game(Scene::Play, false, true, &mut input));
        assert!(ending(&input));
        assert_ne!(
            input.pad.bits, 0,
            "a paused game reads nothing until a move"
        );
    }

    #[test]
    fn the_request_ends_when_play_does() {
        let mut input = Input::default();
        assert!(!end_game(Scene::GameOver, true, false, &mut input));
        assert!(!ending(&input));
    }
}
