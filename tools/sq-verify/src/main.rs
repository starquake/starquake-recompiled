//! Differential tests: runs a routine of the original game in the reference
//! interpreter and the corresponding rewritten Rust from the same starting
//! state, then compares the resulting states.
//!
//! Usage: `sq-verify [ASSETS_DIR]` (needs `starquake.tap` and `48.rom`).
//!
//! The original starts from the player's tape, as the ROM's loader leaves
//! it (`layout::ENTRY_PC`), and the rewrite from the same memory.

use std::path::PathBuf;
use std::rc::Rc;

use starquake::Game;
use starquake::assets::Assets;
use starquake::layout as at;
use zx_runtime::Zx;

struct Env {
    /// The original at its menu, the title tune played out, having run from
    /// the tape's entry with the real ROM: every check starts from here.
    start: Zx,
    assets: Rc<Assets>,
    /// The player's tape, as loaded.
    tape: zx_core::tape::Tape,
}

/// The title screen and menu, where the program's start-up ends.
const MENU: u16 = 0x5E81;

impl Env {
    fn machine(&self) -> Zx {
        self.start.clone()
    }

    fn game(&self, z: &Zx) -> Game {
        Game::from_memory(self.assets.clone(), &z.mem[..])
    }
}

/// Differences between the original's resulting state and the rewrite's,
/// over the parts of the state a test covers.
fn diff(orig: &Game, new: &Game, parts: &[&str]) -> Vec<String> {
    let mut out = Vec::new();
    for &part in parts {
        let (a, b) = match part {
            "display" => {
                let n = (0..orig.display.mem.len())
                    .filter(|&i| orig.display.mem[i] != new.display.mem[i])
                    .count();
                if n > 0 {
                    let i = (0..orig.display.mem.len())
                        .find(|&i| orig.display.mem[i] != new.display.mem[i])
                        .unwrap();
                    out.push(format!(
                        "display: {n} bytes differ, first at +{i:04x} (orig {:02x}, new {:02x})",
                        orig.display.mem[i], new.display.mem[i]
                    ));
                }
                continue;
            }
            "rng" => (format!("{:x?}", orig.rng), format!("{:x?}", new.rng)),
            "colour" => (
                format!("{:02x}", orig.colour),
                format!("{:02x}", new.colour),
            ),
            "room_colours" => (
                format!("{:?}", orig.room_colours),
                format!("{:?}", new.room_colours),
            ),
            "restore" => (
                format!("{:04x} {:02x?}", orig.restore_ptr, orig.restore_mem),
                format!("{:04x} {:02x?}", new.restore_ptr, new.restore_mem),
            ),
            "misc" => (
                format!(
                    "room {} teleporters {:?} footstep {} pickups {} {:?}",
                    orig.room,
                    orig.teleporters,
                    orig.footstep_sound,
                    orig.pickups_in_room,
                    orig.items
                ),
                format!(
                    "room {} teleporters {:?} footstep {} pickups {} {:?}",
                    new.room, new.teleporters, new.footstep_sound, new.pickups_in_room, new.items
                ),
            ),
            "objects" => (format!("{:?}", orig.objects), format!("{:?}", new.objects)),
            "status" => (format!("{:?}", orig.status), format!("{:?}", new.status)),
            "printer" => (format!("{:?}", orig.printer), format!("{:?}", new.printer)),
            "player" => (
                format!("{:?}", &orig.entities[0].0[5..7]),
                format!("{:?}", &new.entities[0].0[5..7]),
            ),
            "entities" => {
                for k in 0..orig.entities.len() {
                    if orig.entities[k] != new.entities[k] {
                        out.push(format!(
                            "entity {k}:\n    orig {:02x?}\n    new  {:02x?}",
                            orig.entities[k].0, new.entities[k].0
                        ));
                    }
                }
                continue;
            }
            "scratch" => (
                format!(
                    "{:02x?} {:02x?} {:02x?}",
                    orig.collision, orig.sound, orig.enemy_cursor
                ),
                format!(
                    "{:02x?} {:02x?} {:02x?}",
                    new.collision, new.sound, new.enemy_cursor
                ),
            ),
            "spawner" => (
                format!("{:x?} {:x?}", orig.spawner, orig.enemy_cache),
                format!("{:x?} {:x?}", new.spawner, new.enemy_cache),
            ),
            "entry" => (
                format!(
                    "{:?} {} {:?} {}",
                    orig.platforms, orig.platform_cursor, orig.saved_position, orig.saved_state
                ),
                format!(
                    "{:?} {} {:?} {}",
                    new.platforms, new.platform_cursor, new.saved_position, new.saved_state
                ),
            ),
            "frames" => (format!("{}", orig.frames), format!("{}", new.frames)),
            "gameover" => (
                format!(
                    "{} {:?} {:02x?}",
                    orig.adventure, orig.score_digits, orig.high_scores
                ),
                format!(
                    "{} {:?} {:02x?}",
                    new.adventure, new.score_digits, new.high_scores
                ),
            ),
            "newgame" => (
                format!(
                    "{:?} cores {:02x?} {} {} vars {} {} {} seed {:04x} {:?}",
                    orig.controls,
                    orig.core_slots,
                    orig.cores_left,
                    orig.cores,
                    orig.var_d2bf,
                    orig.var_d2e9,
                    orig.var_d2ea,
                    orig.seed,
                    orig.bonus
                ),
                format!(
                    "{:?} cores {:02x?} {} {} vars {} {} {} seed {:04x} {:?}",
                    new.controls,
                    new.core_slots,
                    new.cores_left,
                    new.cores,
                    new.var_d2bf,
                    new.var_d2e9,
                    new.var_d2ea,
                    new.seed,
                    new.bonus
                ),
            ),
            "pickups" => (
                format!(
                    "{:?} {:?} {:?} {:?} {} {}",
                    orig.items,
                    orig.bonus,
                    orig.bonus_rooms,
                    orig.unvisited_rooms,
                    orig.pickups_in_room,
                    orig.last_spawn_index
                ),
                format!(
                    "{:?} {:?} {:?} {:?} {} {}",
                    new.items,
                    new.bonus,
                    new.bonus_rooms,
                    new.unvisited_rooms,
                    new.pickups_in_room,
                    new.last_spawn_index
                ),
            ),
            _ => unreachable!("unknown part {part}"),
        };
        if a != b {
            out.push(format!("{part}:\n    orig {a}\n    new  {b}"));
        }
    }
    out
}

fn report(name: &str, failures: &[(String, Vec<String>)], total: usize) -> bool {
    let show = if std::env::var_os("SQ_ALL").is_some() {
        failures.len()
    } else {
        4
    };
    for (case, diffs) in failures.iter().take(show) {
        for d in diffs {
            println!("  {name} {case}: {d}");
        }
    }
    if failures.len() > show {
        println!(
            "  {name}: {} more failing cases not shown (SQ_ALL=1)",
            failures.len() - show
        );
    }
    // A check that ran nothing has proved nothing, so it must not pass: an
    // empty state list used to make a dozen checks report "0/0 cases match"
    // and the whole run succeed.
    if total == 0 {
        println!("{name}: no cases ran");
        return false;
    }
    println!(
        "{name}: {}/{total} cases match",
        total.saturating_sub(failures.len())
    );
    failures.is_empty()
}

/// One per-frame routine to check: its name, the original's address, and the
/// rewrite's equivalent.
type FrameRoutine = (&'static str, u16, fn(&mut Game));

/// A screen drawn once and compared: the same shape.
type Screen = FrameRoutine;

/// Collects a set of states, reporting a panic rather than unwinding out of
/// `main`. An empty set makes every check that uses it report "no cases ran".
fn guarded_states(name: &str, collect: impl FnOnce() -> Vec<Zx>) -> Vec<Zx> {
    match std::panic::catch_unwind(std::panic::AssertUnwindSafe(collect)) {
        Ok(states) => states,
        Err(_) => {
            println!("{name}: PANICKED while collecting states");
            Vec::new()
        }
    }
}

/// Runs one check, turning a panic into a reported failure.
///
/// The hook installed in `main` silences panic messages, so without this a
/// panic in the rewrite unwinds out of `main` with no output at all, taking
/// the remaining checks and the summary line with it.
fn guarded(name: &str, check: impl FnOnce() -> bool) -> bool {
    match std::panic::catch_unwind(std::panic::AssertUnwindSafe(check)) {
        Ok(ok) => ok,
        Err(e) => {
            let msg = e
                .downcast_ref::<String>()
                .map(String::as_str)
                .or_else(|| e.downcast_ref::<&str>().copied())
                .unwrap_or("(no message)");
            println!("{name}: PANICKED: {msg}");
            false
        }
    }
}

fn check_rooms(env: &Env) -> bool {
    let mut failures = Vec::new();
    for room in 0..starquake::assets::ROOM_COUNT as u16 {
        let mut z = env.machine();
        z.mem[0x4000..0x5B00].fill(0);
        z.mem[0x5B20..0x5BC0].fill(0);
        z.write16(at::RESTORE_PTR as u16, at::RESTORE_LIST as u16);
        z.write16(at::ROOM as u16, room);

        let mut g = env.game(&z);
        let done = z.call_until(0xA80A, Some(0xAA30), 5_000_000);
        g.build_room_tiles();
        let parts = [
            "display",
            "rng",
            "colour",
            "room_colours",
            "restore",
            "objects",
        ];
        let mut d = diff(&env.game(&z), &g, &parts);
        if !done {
            d.push("original did not finish".into());
        }
        if !d.is_empty() {
            failures.push((format!("room {room}"), d));
        }
    }
    report("room tiles", &failures, starquake::assets::ROOM_COUNT)
}

/// xorshift, for varied test states.
struct Rng(u64);
impl Rng {
    fn byte(&mut self) -> u8 {
        self.0 ^= self.0 << 13;
        self.0 ^= self.0 >> 7;
        self.0 ^= self.0 << 17;
        (self.0 >> 24) as u8
    }
}

fn check_room_prelude(env: &Env) -> bool {
    let mut failures = Vec::new();
    let cases = 200;
    let mut r = Rng(0x5EED);
    for case in 0..cases {
        let mut z = env.machine();
        // Vary everything the panel shows. Case 0 is the menu as reached.
        if case > 0 {
            for i in 0..6 {
                z.mem[at::SCORE + i] = r.byte() % 10;
                z.mem[at::SCORE_PENDING + i] = if r.byte() < 64 { r.byte() % 30 } else { 0 };
            }
            // The whole byte, not just 0..99: the panel prints lives as two
            // digits, and 100 or more falls off the end of the digit glyphs.
            z.mem[at::LIVES] = r.byte();
            for i in 0..3 {
                z.mem[at::BARS + i] = r.byte();
            }
            for i in 0..4 {
                z.mem[at::INVENTORY + i * 2] = r.byte() % 48;
                z.mem[at::INVENTORY + i * 2 + 1] = if r.byte() < 128 { 0 } else { r.byte() };
            }
            z.mem[at::ENTITIES + 5] = r.byte();
            z.mem[at::ENTITIES + 6] = r.byte();
        }
        let mut g = env.game(&z);
        let done = z.call_until(0xA426, Some(0xA462), 5_000_000);
        g.enter_room_prelude();
        let parts = ["display", "rng", "colour", "status", "printer", "player"];
        let mut d = diff(&env.game(&z), &g, &parts);
        if !done {
            d.push("original did not finish".into());
        }
        if !d.is_empty() {
            failures.push((format!("case {case}"), d));
        }
    }
    report("room prelude (panel)", &failures, cases)
}

/// A machine in the state the original's new-game setup leaves: items
/// scattered, room sets full, fresh seed.
fn new_game_machine(env: &Env) -> Zx {
    let mut z = env.machine();
    // Kempston, so tests can steer BLOB with the joystick byte.
    z.mem[0x5E58] = 1;
    assert!(
        z.call_until(0x629D, Some(0x666D), 50_000_000),
        "new-game setup did not finish"
    );
    z
}

fn check_room_build(env: &Env) -> bool {
    let mut failures = Vec::new();
    let base = new_game_machine(env);
    let mut hangs = 0;
    for room in 0..starquake::assets::ROOM_COUNT as u16 {
        let mut z = env.machine();
        z.mem.copy_from_slice(&base.mem[..]);
        z.mem[0x4000..0x5B00].fill(0);
        z.mem[0x5B20..0x5BC0].fill(0);
        z.write16(at::RESTORE_PTR as u16, at::RESTORE_LIST as u16);
        z.write16(at::ROOM as u16, room);

        let mut g = env.game(&z);
        let done = z.call(0xA7FC, 5_000_000);
        let new = std::panic::catch_unwind(std::panic::AssertUnwindSafe(|| {
            g.build_room();
            g
        }));
        match (done, new) {
            // Both hang (the original loops forever choosing from no spawn points).
            (false, Err(_)) => hangs += 1,
            (true, Ok(g)) => {
                let parts = [
                    "display",
                    "rng",
                    "colour",
                    "room_colours",
                    "restore",
                    "objects",
                    "pickups",
                    "printer",
                ];
                let d = diff(&env.game(&z), &g, &parts);
                if !d.is_empty() {
                    failures.push((format!("room {room}"), d));
                }
            }
            (done, _) => failures.push((
                format!("room {room}"),
                vec![format!(
                    "original finished: {done}, rewrite finished: {}",
                    !done
                )],
            )),
        }
    }
    if hangs > 0 {
        println!("  ({hangs} rooms hang in both: an unplaced item and no spawn points)");
    }
    report(
        "room build with pickups (new game)",
        &failures,
        starquake::assets::ROOM_COUNT,
    )
}

/// Full room entry (`A426` up to the main loop at `A523`), over chains of
/// rooms so the enemy cache is exercised (A → B → A restores A's enemies).
fn check_room_entry(env: &Env) -> bool {
    let mut failures = Vec::new();
    let base = new_game_machine(env);
    let mut r = Rng(0xC0FFEE);
    let chains = 120;
    let mut cases = 0;
    for chain in 0..chains {
        let mut z = env.machine();
        z.mem.copy_from_slice(&base.mem[..]);
        let pick = |r: &mut Rng| loop {
            let room = (r.byte() as u16) << 1 | (r.byte() & 1) as u16;
            if room != 199 {
                break room;
            }
        };
        let (a, b) = (pick(&mut r), pick(&mut r));
        let rooms = [a, b, a, b];
        let mut g = env.game(&z);
        for (step, &room) in rooms.iter().enumerate() {
            cases += 1;
            let frames = (r.byte() as u16) << 8 | r.byte() as u16;
            z.write16(at::FRAMES as u16, frames);
            z.write16(at::ROOM as u16, room);
            z.mem[at::ENTRY_REASON] = 0;
            g.frames = frames as u32;
            g.room = room;
            g.entry_reason = 0;

            let done = z.call_until(0xA426, Some(0xA523), 20_000_000);
            let new = std::panic::catch_unwind(std::panic::AssertUnwindSafe(|| {
                g.enter_room();
                g
            }));
            let name = format!("chain {chain} step {step} (room {room})");
            match (done, new) {
                (true, Ok(new)) => {
                    let orig = env.game(&z);
                    let parts = [
                        "display",
                        "rng",
                        "colour",
                        "room_colours",
                        "restore",
                        "objects",
                        "pickups",
                        "status",
                        "printer",
                        "entities",
                        "spawner",
                        "entry",
                    ];
                    let d = diff(&orig, &new, &parts);
                    if !d.is_empty() {
                        failures.push((name, d));
                        break;
                    }
                    // Continue the chain from the original's state so one
                    // mismatch does not cascade.
                    g = orig;
                }
                (done, _) => {
                    failures.push((
                        name,
                        vec![format!(
                            "original finished: {done}; rewrite panicked or diverged"
                        )],
                    ));
                    break;
                }
            }
        }
    }
    report("room entry with enemies (room chains)", &failures, cases)
}

/// Machine states at the top of the original's main loop during play,
/// with random joystick input, every `every` loop iterations.
fn gameplay_states(env: &Env, count: usize, every: usize) -> Vec<Zx> {
    let mut z = env.machine();
    let mut misses = zx_runtime::Misses::default();
    let key = |name| zx_runtime::keys::Key::by_name(name).unwrap();
    for f in 0..400 {
        z.release_all_keys();
        if (250..253).contains(&f) {
            z.set_key(key("1"), true);
        }
        if (300..303).contains(&f) {
            z.set_key(key("0"), true);
        }
        z.run_frame(zx_runtime::no_code, &mut misses);
    }
    let dirs = ["joy_left", "joy_right", "joy_up", "joy_down", "joy_fire"];
    let mut r = Rng(0xBEEF);
    let mut states = Vec::new();
    let mut i = 0;
    while states.len() < count {
        if i % 10 == 0 {
            z.release_all_keys();
            for _ in 0..r.byte() % 3 {
                z.set_key(key(dirs[r.byte() as usize % dirs.len()]), true);
            }
        }
        if !z.run_until(0xA523, 400) {
            println!(
                "  (gameplay left the main loop after {} states)",
                states.len()
            );
            break;
        }
        if i % every == 0 {
            states.push(z.clone());
        }
        i += 1;
    }
    states
}

/// Checks one per-frame routine against the original over gameplay states.
fn check_frame_routine(
    env: &Env,
    states: &[Zx],
    name: &str,
    addr: u16,
    run: fn(&mut Game),
) -> bool {
    let mut failures = Vec::new();
    for (n, state) in states.iter().enumerate() {
        let mut z = state.clone();
        let mut g = env.game(&z);
        let done = z.call(addr, 5_000_000);
        run(&mut g);
        let parts = [
            "display", "entities", "entry", "objects", "rng", "restore", "scratch", "status",
        ];
        let mut d = diff(&env.game(&z), &g, &parts);
        let frames = z.read16(at::FRAMES as u16) as u32 | (z.mem[at::FRAMES + 2] as u32) << 16;
        if frames != g.frames {
            d.push(format!("frames: orig {frames} new {}", g.frames));
        }
        if !done {
            d.push("original did not finish".into());
        }
        if !d.is_empty() {
            failures.push((format!("state {n} (room {})", g.room), d));
        }
    }
    report(name, &failures, states.len())
}

/// The gameplay states plus, for each, a variant with an active enemy
/// moved onto BLOB (so touching and killing get tested).
fn with_contact_variants(states: &[Zx]) -> Vec<Zx> {
    let mut out = states.to_vec();
    for s in states {
        let base = at::ENTITIES;
        let count = s.mem[at::SPAWN_COUNT] as usize;
        if let Some(k) = (1..=count.min(4)).find(|&k| s.mem[base + k * 32 + 0x15] == 1) {
            let mut v = s.clone();
            v.mem[base + k * 32 + 5] = v.mem[base + 5].wrapping_add(4);
            v.mem[base + k * 32 + 6] = v.mem[base + 6];
            out.push(v.clone());
            // Most enemy graphics are harmless (they only drain energy);
            // force the two deadly kinds too.
            for graphic in [0xB2C8u16, 0xB308] {
                let mut d = v.clone();
                d.mem[base + k * 32 + 7] = graphic as u8;
                d.mem[base + k * 32 + 8] = (graphic >> 8) as u8;
                out.push(d);
            }
        }
    }
    out
}

/// The enemy update over gameplay states. If the original reaches the
/// death routine, the rewrite must report the same death reason.
fn check_enemies(env: &Env, states: &[Zx]) -> bool {
    let mut failures = Vec::new();
    let mut deaths = 0;
    let states = with_contact_variants(states);
    for (n, state) in states.iter().enumerate() {
        let mut z = state.clone();
        let mut g = env.game(&z);
        let done = z.call_until(0xA01B, Some(0xC350), 5_000_000);
        let died = (z.pc == 0xC350).then_some(z.a);
        let new_died = g.update_enemies();
        let parts = [
            "display", "entities", "status", "printer", "rng", "spawner", "scratch",
        ];
        let mut d = diff(&env.game(&z), &g, &parts);
        if died != new_died {
            d.push(format!("death: orig {died:?} new {new_died:?}"));
        }
        if died.is_some() {
            deaths += 1;
        }
        if !done {
            d.push("original did not finish".into());
        }
        if !d.is_empty() {
            failures.push((format!("state {n} (room {})", g.room), d));
        }
    }
    println!("  ({deaths} states end in BLOB being killed)");
    report("enemies (A01B)", &failures, states.len())
}

/// Variants of gameplay states that drive BLOB into the less common paths:
/// every joystick input, standing on each marker in the room, at each room
/// edge, and in the lifted and hovering states.
fn blob_variants(states: &[Zx]) -> Vec<Zx> {
    const RIGHT: u8 = 1;
    const LEFT: u8 = 2;
    const DOWN: u8 = 4;
    const UP: u8 = 8;
    const FIRE: u8 = 0x10;
    let blob = at::ENTITIES;
    let with = |s: &Zx, input: u8, f: &dyn Fn(&mut Zx)| {
        let mut v = s.clone();
        v.release_all_keys();
        v.kempston = input;
        f(&mut v);
        v
    };
    let mut out = Vec::new();
    for s in states {
        for input in [
            0,
            RIGHT,
            LEFT,
            DOWN,
            UP,
            FIRE,
            RIGHT | FIRE,
            LEFT | UP,
            UP | RIGHT,
            DOWN | LEFT,
        ] {
            out.push(with(s, input, &|_| {}));
        }
        let end = s.read16(at::objects::MARKERS_END as u16) as usize;
        for m in (at::objects::MARKERS..end.min(at::objects::MARKERS + 66)).step_by(3) {
            let (x, y) = (s.mem[m], s.mem[m + 1]);
            for input in [UP, RIGHT, LEFT, 0] {
                out.push(with(s, input, &|v| {
                    v.mem[blob + 5] = x;
                    v.mem[blob + 6] = y;
                }));
            }
            // Walking onto markers that need an exact position and a
            // push sideways (doors, teleporter booths, tubes).
            for (dx, input) in [(2u8, RIGHT), (0u8.wrapping_sub(2), LEFT)] {
                out.push(with(s, input, &|v| {
                    v.mem[blob + 5] = x.wrapping_sub(dx);
                    v.mem[blob + 6] = y;
                }));
            }
            // Falling at full speed onto the marker (crumbling floor).
            out.push(with(s, 0, &|v| {
                v.mem[blob + 5] = x;
                v.mem[blob + 6] = y;
                v.mem[blob + 0x11] = 0x10;
            }));
        }
        for (x, y, input) in [(0xF1, None, RIGHT), (0x01, None, LEFT), (0xF0, None, RIGHT)] {
            out.push(with(s, input, &|v| {
                v.mem[blob + 5] = x;
                if let Some(y) = y {
                    v.mem[blob + 6] = y;
                }
            }));
        }
        for y in [0x0D, 0x90, 0x15, 0x17] {
            out.push(with(s, 0, &|v| v.mem[blob + 6] = y));
        }
        // Low in the room with Down held, which is what makes `build_platform`
        // probe 64 + 32 bytes past the cell it starts from: below row 22 that
        // lands in the restore list, past the display's guard row. The
        // platform must not already be held and the bar must have charge, or
        // the branch returns early.
        for y in [0x0E, 0x0F, 0x10, 0x0C, 0x08] {
            out.push(with(s, DOWN, &|v| {
                v.mem[blob + 6] = y;
                v.mem[blob + 0x14] = 0;
                v.mem[at::BARS + 1] = 0x40;
            }));
        }
        for input in [UP, DOWN, RIGHT | FIRE, LEFT | DOWN, FIRE] {
            out.push(with(s, input, &|v| v.mem[blob + 0x0A] = 2));
        }
        out.push(with(s, RIGHT, &|v| v.mem[blob + 0x0A] = 1));
    }
    out
}

/// BLOB's frame (`C552`, after the display work) over gameplay states,
/// with the input the original saw.
fn check_blob(env: &Env, states: &[Zx]) -> bool {
    use starquake::blob::Outcome;
    let mut failures = Vec::new();
    let mut outcomes = std::collections::BTreeMap::<String, usize>::new();
    let states = blob_variants(states);
    for (n, state) in states.iter().enumerate() {
        let mut z = state.clone();
        let mut g = env.game(&z);
        let input = input_of(&z);
        let (done, sounds) =
            run_noting_sounds(&mut z, 0xC552, &[0xC350, 0xA412, 0x5E29], 5_000_000);
        let orig = match z.pc {
            0xC350 => Outcome::Died(z.a),
            0xA412 => Outcome::Modal(modal_at(&z)),
            0x5E29 => Outcome::Quit,
            _ if z.a < 0x64 => Outcome::NewRoom(z.a),
            _ => Outcome::Continue,
        };
        let new = g.blob_control(&input);
        *outcomes.entry(format!("{orig:?}")).or_default() += 1;
        let parts = [
            "display", "entities", "status", "printer", "rng", "objects", "restore", "entry",
            "scratch", "pickups", "misc",
        ];
        let mut d = diff(&env.game(&z), &g, &parts);
        if sounds != g.effects {
            d.push(format!("sounds: orig {sounds:02x?} new {:02x?}", g.effects));
        }
        let same = orig == new;
        if !same {
            d.push(format!("outcome: orig {orig:?} new {new:?}"));
        }
        if !done {
            d.push("original did not finish".into());
        }
        if !d.is_empty() {
            failures.push((format!("state {n} (room {})", g.room), d));
        }
    }
    println!("  outcomes: {outcomes:?}");
    report("BLOB (C5BD)", &failures, states.len())
}

/// The death sequence for several reasons, with and without lives left.
/// The original is entered as if called from the main loop and runs with
/// real interrupts until it re-enters the room (`A410`) or the game ends.
fn check_death(env: &Env, states: &[Zx]) -> bool {
    let mut failures = Vec::new();
    let mut cases = 0;
    for (n, state) in states.iter().enumerate().step_by(5) {
        for reason in [0x01u8, 0x02, 0x10, 0x11] {
            for last_life in [false, true] {
                cases += 1;
                let mut z = state.clone();
                if last_life {
                    z.mem[at::LIVES] = 0;
                }
                let mut g = env.game(&z);
                z.push(0x0000);
                z.push(0xA53A);
                z.pc = 0xC350;
                z.a = reason;
                z.halted = false;
                let mut sounds = Vec::new();
                let ok = z.run_until_any_with(&[0xA410, 0x0000], 20_000, |z| {
                    if z.pc == SOUND_ROUTINE {
                        sounds.push(z.a);
                    }
                });
                let orig_continues = z.pc == 0xA410;
                let mut host = SoundLog::default();
                let continues = g.death_sequence(reason, &mut host);
                let parts = [
                    "display", "entities", "status", "printer", "rng", "objects", "restore",
                    "entry", "scratch", "pickups",
                ];
                let mut d = diff(&env.game(&z), &g, &parts);
                let asked = host.all(&g);
                if sounds != asked {
                    d.push(format!("sounds: orig {sounds:02x?} new {asked:02x?}"));
                }
                // `frames` is deliberately not compared here. The death
                // sequence is mostly blocking sound, and the host is told
                // `1 + busy / FRAME_T` frames passed, rounding up once per
                // effect, where the original counts the interrupts that
                // actually arrived during it. Measured, the two drift by one
                // to eight frames over a death (orig 413 against 406, 504
                // against 496), so a comparison here reports the rounding,
                // not a regression. Anything that consumes whole frames
                // without sound -- the 80-frame animation, the 50-frame
                // pause -- is already covered by the state it leaves behind.
                if orig_continues != continues {
                    d.push(format!("continues: orig {orig_continues} new {continues}"));
                }
                if !ok {
                    d.push("original did not finish".into());
                }
                if !d.is_empty() {
                    failures.push((
                        format!("state {n} reason {reason:#04x} last life {last_life}"),
                        d,
                    ));
                }
            }
        }
    }
    report("death sequence (C350)", &failures, cases)
}

/// The end-of-game screen (`6730` up to its tune), over varied scores,
/// times and core pieces.
fn check_game_over(env: &Env, states: &[Zx]) -> bool {
    let mut failures = Vec::new();
    let mut cases = 0;
    let mut r = Rng(0x60AD);
    for (n, state) in states.iter().enumerate().step_by(20) {
        for _ in 0..4 {
            cases += 1;
            let mut z = state.clone();
            for i in 0..6 {
                z.mem[at::SCORE + i] = r.byte() % 10;
            }
            z.mem[at::CORES_LEFT] = r.byte() % 10;
            z.write16(at::FRAMES as u16, (r.byte() as u16) << 8 | r.byte() as u16);
            z.mem[at::FRAMES + 2] = r.byte() % 4;
            let mut g = env.game(&z);
            // Without interrupts, so the original's frame counter (and so
            // the time it prints) stays where the rewrite's is.
            let ok = z.call_until(0x6730, Some(0x685F), 20_000_000);
            g.game_over_screen();
            let parts = ["display", "status", "misc", "gameover", "printer", "rng"];
            let mut d = diff(&env.game(&z), &g, &parts);
            if !ok {
                d.push("original did not finish".into());
            }
            if !d.is_empty() {
                failures.push((format!("state {n}"), d));
            }
        }
    }
    report("game over screen (6730)", &failures, cases)
}

/// A gamepad pressed through a game's end with a top score, between key
/// presses of A: it may end the tunes, but it types none of the initials
/// (#123), so they come out as the keyboard's AAA.
struct PadAndKeys {
    frame: u32,
}

impl starquake::host::Host for PadAndKeys {
    fn frame(&mut self, _game: &Game) -> (starquake::controls::Input, u32) {
        self.frame += 1;
        let mut input = starquake::controls::Input::default();
        match self.frame % 4 {
            0 => {
                input.pad.bits = 0x1F;
                input.pad.start = true;
            }
            2 => input.press_key((0xFD, 0)),
            _ => {}
        }
        (input, 1)
    }
}

fn check_pad_types_nothing(env: &Env, states: &[Zx]) -> bool {
    let mut failures = Vec::new();
    let mut z = states[0].clone();
    z.mem[at::SCORE..at::SCORE + 6].fill(9);
    let mut g = env.game(&z);
    g.game_over(&mut PadAndKeys { frame: 0 });
    if g.high_scores[..3] != *b"AAA" {
        failures.push((
            "initials".into(),
            vec![format!(
                "top entry {:?}",
                String::from_utf8_lossy(&g.high_scores[..10])
            )],
        ));
    }
    report("gamepad types no initials (#123)", &failures, 1)
}

/// Pauses a game with its pause key, then asks it to end as the window's
/// "End this game" does while paused (#125): A S D F G with a move. Stops the
/// run if the game is still going long after.
struct EndWhilePaused {
    frame: u32,
    pause: (u8, u8),
    paused_seen: bool,
}

impl starquake::host::Host for EndWhilePaused {
    fn frame(&mut self, game: &Game) -> (starquake::controls::Input, u32) {
        self.frame += 1;
        self.paused_seen |= game.paused;
        assert!(self.frame < 2_000, "the game did not end");
        let mut input = starquake::controls::Input::default();
        match self.frame {
            10..=12 => input.press_key(self.pause),
            40.. if game.paused => {
                input.keys[1] &= !0x1F;
                input.pad.bits = 0x01;
            }
            _ => {}
        }
        (input, 1)
    }
}

fn check_end_while_paused(env: &Env, states: &[Zx]) -> bool {
    let mut failures = Vec::new();
    let mut g = env.game(&states[0]);
    let mut host = EndWhilePaused {
        frame: 0,
        pause: g.controls.pause,
        paused_seen: false,
    };
    g.play(&mut host);
    if !host.paused_seen {
        failures.push(("pause".into(), vec!["the game never paused".into()]));
    }
    if host.frame > 60 {
        failures.push((
            "end".into(),
            vec![format!("ended only at frame {}", host.frame)],
        ));
    }
    report("ending a paused game (#125)", &failures, 1)
}

/// The lift in room 244's right-hand shaft, walked into from the left
/// (#117). The original, run on the tape, lifts BLOB from (200, 39) to the
/// top of the shaft at (200, 111); the rewrite used to leave him standing at
/// its foot, having painted the lift cell under him white with the
/// snapshot's damaged sprite colouring.
fn check_lift(env: &Env) -> bool {
    use starquake::entities::field::{X, Y};
    let mut failures = Vec::new();
    let mut g = env.game(&new_game_machine(env));
    g.room = 244;
    g.entry_reason = 0;
    g.enter_room();
    g.entities[0].0[X] = 180;
    g.entities[0].0[Y] = 39;
    let input = starquake::controls::Input {
        kempston: 0x01,
        ..Default::default()
    };
    for _ in 0..150 {
        g.play_frame(&input);
    }
    let at = (g.entities[0].0[X], g.entities[0].0[Y]);
    if g.room != 244 || at != (200, 111) {
        failures.push((
            "room 244, walking right from (180, 39)".into(),
            vec![format!(
                "BLOB at {at:?} in room {}, the original at (200, 111)",
                g.room
            )],
        ));
    }
    report("lift boarded walking right (#117)", &failures, 1)
}

/// Notes, frame by frame, how many different pictures a frame's blocking
/// effects were asked for over (#116).
struct Pictures {
    most: usize,
}

impl starquake::host::Host for Pictures {
    fn frame(&mut self, game: &Game) -> (starquake::controls::Input, u32) {
        let mut distinct: Vec<&[u8]> = Vec::new();
        for p in &game.effect_pictures {
            if !distinct.contains(&&p.mem[..]) {
                distinct.push(&p.mem[..]);
            }
        }
        self.most = self.most.max(distinct.len());
        (
            starquake::controls::Input::default(),
            game.frame_sound().frames,
        )
    }
}

/// A security door's screen flashes its code items while its beeps play:
/// the frame those effects stretch carries a picture for each (#116), so a
/// window can show the flashing rather than only where it ended.
fn check_effect_pictures(env: &Env) -> bool {
    let mut failures = Vec::new();
    let mut g = env.game(&new_game_machine(env));
    g.room = 176;
    let mut host = Pictures { most: 0 };
    g.run_modal(starquake::blob::Modal::SecurityDoor, &mut host);
    if host.most < 10 {
        failures.push((
            "security door, room 176".into(),
            vec![format!(
                "at most {} different pictures in a frame's effects",
                host.most
            )],
        ));
    }
    report("pictures under blocking effects (#116)", &failures, 1)
}

/// The core room: walking in carrying pieces that fit holes in the core.
/// The original is run from the room entry to where it leaves for room 198.
fn check_core_room(env: &Env) -> bool {
    let base = new_game_machine(env);
    // "newgame" carries core_slots, cores and cores_left, and "misc" the room
    // number: the three things delivering a piece exists to change.
    let parts = [
        "display", "entities", "status", "rng", "objects", "restore", "pickups", "newgame", "misc",
    ];
    let mut failures = Vec::new();
    let mut cases = 0;

    for carried in 1..=3usize {
        cases += 1;
        let case = format!("{carried} piece(s) carried");
        let mut z = base.clone();
        z.write16(at::ROOM as u16, 199);
        z.mem[at::ENTRY_REASON] = 0;

        // Carry pieces that fit the first holes still missing, and mark the
        // matching items as carried (a carried item's row is 2 + its slot).
        let missing: Vec<u8> = (0..9)
            .filter(|&i| z.mem[at::CORE_SLOTS + i] >= 0x80)
            .map(|i| z.mem[at::CORE_SLOTS + i].wrapping_sub(0x80))
            .collect();
        for s in 0..4 {
            z.mem[at::INVENTORY + s * 2] = 0;
            z.mem[at::INVENTORY + s * 2 + 1] = 0;
        }
        for (s, &graphic) in missing.iter().take(carried).enumerate() {
            z.mem[at::INVENTORY + s * 2] = graphic;
            z.mem[at::INVENTORY + s * 2 + 1] = 0x47;
            z.mem[at::ITEMS + s * 4 + 1] = 2 + s as u8;
            z.mem[at::ITEMS + s * 4 + 3] = graphic;
        }

        let mut g = env.game(&z);
        let mut sounds = Vec::new();
        let watch = |z: &Zx, sounds: &mut Vec<u8>| {
            if z.pc == SOUND_ROUTINE {
                sounds.push(z.a);
            }
        };
        if !z
            .call_until_any_with(0xA426, &[0xA6C1], 20_000_000, |z| watch(z, &mut sounds))
            .0
        {
            let at = format!("original did not reach the core room (pc {:04x})", z.pc);
            failures.push((case, vec![at]));
            continue;
        }
        // The core room's sound and animation end each frame on the 50 Hz
        // interrupt, so from here the original is run like real hardware.
        z.t = 0;
        z.int_pending = false;
        z.iff1 = true;
        if !z.run_until_any_with(&[0xA410], 5000, |z| watch(z, &mut sounds)) {
            let at = format!("original did not finish (pc {:04x})", z.pc);
            failures.push((case, vec![at]));
            continue;
        }
        g.enter_room();
        let mut host = SoundLog::default();
        let finished = g.core_room(&mut host);
        let mut d = diff(&env.game(&z), &g, &parts);
        let asked = host.all(&g);
        if sounds != asked {
            d.push(format!("sounds: orig {sounds:02x?} new {asked:02x?}"));
        }
        if finished {
            d.push("the rewrite ended the game; the original came back".into());
        }
        if !d.is_empty() {
            failures.push((case, d));
        }
    }
    report("core room (A6C1)", &failures, cases)
}

/// The tunes. The original's player turns interrupts off itself, so each
/// tune can simply be called and timed. Matching its total length exactly
/// means every branch of every half-cycle went the same way, which is what
/// sets the pitch, the buzz and the tempo.
fn check_music(env: &Env) -> bool {
    /// The player itself. It is called with the tune's address in HL, and
    /// `call_until_any_timed` supplies the return address, so there is no
    /// stub to subtract afterwards.
    ///
    /// There used to be one, three instructions at 0x5B20. That is inside
    /// the contended sixteen kilobytes, so once the ULA was modelled the
    /// stub cost more than the constant being subtracted for it, and the
    /// measurement carried the harness's own delays into the answer.
    const PLAYER: u16 = 0xD9DE;
    const TUNES: usize = 0x65F4;

    let mut failures = Vec::new();
    let mut cases = 0;
    for tune in 1..=5u8 {
        cases += 1;
        let mut z = env.machine();
        let entry = TUNES + tune as usize * 2;
        let addr = z.mem[entry] as u16 | (z.mem[entry + 1] as u16) << 8;
        z.set_hl(addr);
        z.t = 0;
        let case = format!("tune {tune} at {addr:04x}");
        let (done, original) = z.call_until_any_timed(PLAYER, &[], 2_000_000_000);
        if !done {
            failures.push((case, vec!["original did not finish".into()]));
            continue;
        }
        let (edges, total) = starquake::music::tune(&env.assets.ram, addr as usize);
        let seconds =
            total as f64 / (starquake::host::FRAMES_PER_SECOND * starquake::sound::FRAME_T) as f64;
        println!(
            "  tune {tune}: {seconds:.1}s, {} speaker changes",
            edges.len()
        );
        if original != total {
            let d = format!(
                "length: orig {original} new {total} (off by {})",
                total as i64 - original as i64
            );
            failures.push((case, vec![d]));
        }
    }
    report("music (D9DE)", &failures, cases)
}

/// Measures the ROM routine the music player uses to work out how long a
/// note lasts. The listing only covers the game's own memory, so this runs
/// the real ROM and reports what it computes.
fn probe(env: &Env) {
    // The last pairs swap the operands and sweep the multiplicand's bits, to
    // show which side the cost follows.
    for (hl, de) in [
        (0x0100u16, 0x0020u16),
        (0x1234, 0x0005),
        (0x00FF, 0x00FF),
        (0x0040, 0x0100),
        (0x0005, 0x1234),
        (0x001F, 0x0064),
        (0x003F, 0x0064),
        (0x007F, 0x0064),
        (0x00FF, 0x0064),
        (0xFFFF, 0x0001),
    ] {
        let mut z = env.machine();
        let stub = 0x5B20u16;
        let code = [
            0x21,
            hl as u8,
            (hl >> 8) as u8,
            0x11,
            de as u8,
            (de >> 8) as u8,
            0xCD,
            0xA9,
            0x30,
            0xC9,
        ];
        for (i, b) in code.iter().enumerate() {
            z.mem[stub as usize + i] = *b;
        }
        z.t = 0;
        let ok = z.call(stub, 5_000_000);
        // The stub around the call costs 10 + 10 + 17 + 10.
        let body = z.t.saturating_sub(47);
        println!(
            "30a9: hl={hl:#06x} ({} bits) de={de:#06x} -> {:#06x} (hl*de={:#06x}) body={body} \
             predicted={} ok={ok}",
            hl.count_ones(),
            z.hl(),
            hl.wrapping_mul(de),
            931 + 13 * hl.count_ones()
        );
    }
}

/// Runs the original from `start` exactly as [`Zx::call_until_any`] would,
/// but also notes every call to the blocking sound routine and the effect it
/// was asked for. Those calls leave almost no trace in memory, so comparing
/// the sequence is the only way to know the rewrite asks for the same sounds
/// in the same places.
fn run_noting_sounds(z: &mut Zx, start: u16, stops: &[u16], max: u64) -> (bool, Vec<u8>) {
    const SOUND: u16 = 0xD7C0;
    let mut ids = Vec::new();
    let (done, _) = z.call_until_any_with(start, stops, max, |z| {
        if z.pc == SOUND {
            ids.push(z.a);
        }
    });
    (done, ids)
}

/// The blocking sound routine. Its calls leave nothing behind in memory, so
/// the only way to compare them is to watch the original run.
const SOUND_ROUTINE: u16 = 0xD7C0;

/// A host that plays nothing and remembers every blocking sound the rewrite
/// asks for. `sync` clears `effects` each frame, so a sequence that spans
/// frames -- a death, a core piece going in -- has to be collected as it goes.
#[derive(Default)]
struct SoundLog {
    inner: starquake::host::NullHost,
    ids: Vec<u8>,
}

impl starquake::host::Host for SoundLog {
    fn frame(&mut self, game: &Game) -> (starquake::controls::Input, u32) {
        self.ids.extend_from_slice(&game.effects);
        self.inner.frame(game)
    }
}

impl SoundLog {
    /// Everything asked for, including whatever was pushed after the last
    /// frame boundary and so never reached the host.
    fn all(&self, game: &Game) -> Vec<u8> {
        let mut ids = self.ids.clone();
        ids.extend_from_slice(&game.effects);
        ids
    }
}

/// Which screen the original is about to run, from the address it will
/// return to. `A412` is reached by `call` from four places, and treating
/// them all alike let the rewrite answer "security door" to a pyramid and
/// still pass.
fn modal_at(z: &Zx) -> starquake::blob::Modal {
    use starquake::blob::Modal;
    match z.read16(z.sp) {
        0xCCFC | 0xCD2A => Modal::Cheops,
        0xCED4 => Modal::TeleportBooth,
        _ => Modal::SecurityDoor,
    }
}

/// The machine's input, as the game reads it.
fn input_of(machine: &Zx) -> starquake::controls::Input {
    starquake::controls::Input {
        keys: machine.keys,
        kempston: machine.kempston,
        // The original has no gamepad.
        pad: starquake::controls::PadInput::default(),
    }
}

/// The screens drawn in one pass: the intro, and the high-score table. Both
/// are stopped where the original starts playing its tune.
fn check_screens(env: &Env) -> bool {
    let base = new_game_machine(env);
    let parts = ["display", "printer", "rng", "restore"];
    let mut failures = Vec::new();
    let screens: [Screen; 2] = [
        ("intro (666D)", 0x666D, Game::intro_screen),
        ("core of heroes (654B)", 0x654B, Game::core_of_heroes_screen),
    ];

    for (name, start, draw) in screens {
        let mut z = base.clone();
        let mut g = env.game(&base);
        if !z.call_until(start, Some(0x6600), 20_000_000) {
            failures.push((name.to_string(), vec!["original did not finish".into()]));
            continue;
        }
        draw(&mut g);
        let d = diff(&env.game(&z), &g, &parts);
        if !d.is_empty() {
            failures.push((name.to_string(), d));
        }
    }
    report("screens", &failures, screens.len())
}

/// The menu screens, each drawn once: the original is stopped as soon as it
/// has drawn one, before it starts waiting for the player. (The loops around
/// them run at their own speed in the original, so only the drawing can be
/// compared.)
fn check_menu(env: &Env) -> bool {
    let base = new_game_machine(env);
    let parts = ["display", "printer", "rng", "restore"];
    let mut failures = Vec::new();
    let mut cases = 0;

    // The rewrite spreads this many turns of the menu loop over a second.
    // It was hand-copied from `sq-verify menu`; measuring it here makes it a
    // checked number like every other.
    cases += 1;
    let (turns, left) = measure_menu_rate(env);
    if left {
        failures.push((
            "loop rate".to_string(),
            vec!["the menu loop ended early".into()],
        ));
    } else if turns != starquake::menu::TURNS_PER_SECOND as u64 {
        failures.push((
            "loop rate".to_string(),
            vec![format!(
                "orig {turns} turns/s, rewrite paced at {}",
                starquake::menu::TURNS_PER_SECOND
            )],
        ));
    }

    // The title screen, for each control method: the highlighted option and
    // the key names beside it change with it.
    for method in 1..=5u8 {
        cases += 1;
        let mut z = base.clone();
        z.mem[0x5E58] = method;
        // Stop where the original would start playing its tune.
        if !z.call_until(0x5E81, Some(0x5ED1), 20_000_000) {
            failures.push((
                format!("title, method {method}"),
                vec!["original did not finish".into()],
            ));
            continue;
        }
        let mut g = env.game(&base);
        g.control_method = method;
        g.title_draw();
        let d = diff(&env.game(&z), &g, &parts);
        if !d.is_empty() {
            failures.push((format!("title, method {method}"), d));
        }
    }

    // The keyboard drawn by the define-keys screen.
    cases += 1;
    let mut z = base.clone();
    if z.call_until(0x6194, Some(0x61A8), 20_000_000) {
        let mut g = env.game(&base);
        g.define_keys_draw();
        let d = diff(&env.game(&z), &g, &parts);
        if !d.is_empty() {
            failures.push(("define keys".to_string(), d));
        }
    } else {
        failures.push((
            "define keys".to_string(),
            vec!["original did not finish".into()],
        ));
    }

    // The quit confirmation.
    cases += 1;
    let mut z = base.clone();
    if z.call_until(0x6060, Some(0x6099), 20_000_000) {
        let mut g = env.game(&base);
        g.quit_screen();
        let d = diff(&env.game(&z), &g, &parts);
        if !d.is_empty() {
            failures.push(("quit".to_string(), d));
        }
    } else {
        failures.push(("quit".to_string(), vec!["original did not finish".into()]));
    }

    report("menu (5E81)", &failures, cases)
}

/// Rooms with a marker of kind `kind`, found by building every room.
fn rooms_with_marker(env: &Env, base: &Zx, kind: u8) -> Vec<(u16, u8, u8)> {
    let mut out = Vec::new();
    for room in 0..512u16 {
        if room == 199 {
            continue;
        }
        let mut g = env.game(base);
        g.room = room;
        g.build_room_tiles();
        for m in &g.objects.markers {
            if m.kind == kind {
                out.push((room, m.x, m.y));
            }
        }
    }
    out
}

/// Security doors: BLOB walks into doors all over the map, with no key
/// (refused) and with the master key (let through).
fn check_security_doors(env: &Env) -> bool {
    use starquake::play::FrameEvent;
    let base = new_game_machine(env);
    let doors = rooms_with_marker(env, &base, 0);
    println!("  security doors found: {}", doors.len());
    let mut failures = Vec::new();
    let mut triggered = 0;
    // Every position tried is a comparison, whether or not it reaches a door
    // screen. Counting only the ones that did let the failure count exceed
    // the total and underflow the "x/y match" line.
    let mut cases = 0;
    for &(room, mx, my) in doors.iter().take(40) {
        let mut z = base.clone();
        z.write16(at::ROOM as u16, room);
        z.mem[at::ENTRY_REASON] = 0;
        if !z.call_until(0xA426, Some(0xA523), 20_000_000) {
            continue;
        }
        z.t = 0;
        z.int_pending = false;
        // Walking into a door only reaches it when the door blocks BLOB and
        // he is standing, so sweep the positions around the marker.
        for dy in [0i32, -1, 1, -4, 4, -8, 8] {
            for dx in [0i32, -2, 2] {
                for input in [1u8, 2] {
                    for master_key in [false, true] {
                        cases += 1;
                        let mut v = z.clone();
                        v.mem[at::ENTITIES + 5] = mx.wrapping_add(dx as u8);
                        v.mem[at::ENTITIES + 6] = my.wrapping_add(dy as u8);
                        if master_key {
                            v.mem[at::INVENTORY] = 0x0F;
                            v.mem[at::INVENTORY + 1] = 5;
                        }
                        v.release_all_keys();
                        v.kempston = input;
                        let mut g = env.game(&v);
                        let host_input = input_of(&v);
                        g.frame_display();
                        let event = g.play_logic(&host_input);
                        let mut sounds = Vec::new();
                        let ok = v.run_until_any_with(&[0xA426, 0xA523], 20_000, |v| {
                            if v.pc == SOUND_ROUTINE {
                                sounds.push(v.a);
                            }
                        });
                        // The door screen is the only thing that re-enters a
                        // room with reason 3.
                        let orig_door = v.pc == 0xA426 && v.mem[at::ENTRY_REASON] == 3;
                        let new_door = matches!(event, FrameEvent::Modal(_));
                        let case =
                            format!("room {room} at {dx},{dy} input {input} key {master_key}");
                        if orig_door != new_door {
                            failures.push((
                                case,
                                vec![format!("door screen: orig {orig_door} new {new_door}")],
                            ));
                            continue;
                        }
                        if !new_door {
                            continue;
                        }
                        triggered += 1;
                        let FrameEvent::Modal(m) = event else {
                            unreachable!()
                        };
                        let mut host = SoundLog::default();
                        let reason = g.run_modal(m, &mut host);
                        let parts = [
                            "display", "entities", "status", "printer", "rng", "objects",
                            "restore", "scratch", "pickups",
                        ];
                        let mut d = diff(&env.game(&v), &g, &parts);
                        let asked = host.all(&g);
                        if sounds != asked {
                            d.push(format!("sounds: orig {sounds:02x?} new {asked:02x?}"));
                        }
                        if v.mem[at::ENTRY_REASON] != reason {
                            d.push(format!(
                                "reason: orig {} new {reason}",
                                v.mem[at::ENTRY_REASON]
                            ));
                        }
                        if !ok {
                            d.push("original did not finish".into());
                        }
                        if !d.is_empty() {
                            failures.push((case, d));
                        }
                    }
                }
            }
        }
    }
    println!("  door screens reached: {triggered} of {cases} positions tried");
    report("security doors (D5FD)", &failures, cases)
}

/// New-game setup (`629D` up to the intro at `666D`) for every control
/// method and a range of frame counts (the seed).
/// A gamepad reaches the game in every control method. For each method,
/// every combination of directions and fire pressed through
/// `Controls::press` has to read back as the same bits.
///
/// Not a comparison with the original, but it needs the real method tables,
/// which only the tape has, so it runs here rather than as a unit test.
fn check_gamepad_methods(env: &Env) -> bool {
    let mut failures = Vec::new();
    let mut cases = 0;
    for method in 1..=5u8 {
        let mut game = env.game(&env.machine());
        game.new_game(method);
        for bits in 0..0x20u8 {
            cases += 1;
            let mut input = starquake::controls::Input::default();
            input.pad.bits = bits;
            let got = game.controls.read(&input);
            let typed = starquake::controls::key_code(&game.assets.ram, &input);
            if got != bits || typed != 0 {
                failures.push((
                    format!("method {method}, bits {bits:#04x}"),
                    vec![format!("read back {got:#04x}, typed {typed:#04x}")],
                ));
            }
        }
    }
    report("gamepad in every control method", &failures, cases)
}

fn check_new_game(env: &Env) -> bool {
    let mut failures = Vec::new();
    let mut cases = 0;
    let mut r = Rng(0x5A11);
    for method in 1..=5u8 {
        for _ in 0..40 {
            cases += 1;
            let mut z = env.machine();
            let frames = (r.byte() as u16) << 8 | r.byte() as u16;
            z.mem[0x5E58] = method;
            z.write16(at::FRAMES as u16, frames);
            let mut g = env.game(&z);
            let done = z.call_until(0x629D, Some(0x666D), 20_000_000);
            g.new_game(method);
            let parts = [
                "display", "entities", "status", "rng", "pickups", "misc", "frames", "newgame",
                "entry",
            ];
            let mut d = diff(&env.game(&z), &g, &parts);
            if !done {
                d.push("original did not finish".into());
            }
            if !d.is_empty() {
                failures.push((format!("method {method} frames {frames}"), d));
            }
        }
    }
    report("new game (629D)", &failures, cases)
}

/// States at the top of the main loop just after entering random rooms all
/// over the map (from a new game), so that every kind of scenery and
/// marker turns up.
fn room_tour_states(env: &Env, count: usize) -> Vec<Zx> {
    let base = new_game_machine(env);
    let mut r = Rng(0x70C4);
    let mut states = Vec::new();
    while states.len() < count {
        let room = (r.byte() as u16) << 1 | (r.byte() & 1) as u16;
        if room == 199 {
            continue;
        }
        let mut z = base.clone();
        z.write16(at::FRAMES as u16, (r.byte() as u16) << 8 | r.byte() as u16);
        z.write16(at::ROOM as u16, room);
        z.mem[at::ENTRY_REASON] = 0;
        z.kempston = 0;
        if z.call_until(0xA426, Some(0xA523), 20_000_000) {
            // The call ran without frame timing; start a fresh frame.
            z.t = 0;
            z.int_pending = false;
            z.halted = false;
            states.push(z);
        }
    }
    states
}

/// The map's openings (#2) against where BLOB actually goes.
///
/// From each state the rewrite plays on under random joystick input, and
/// every time BLOB walks, falls, flies or takes a wall passage into the next
/// room, the edge he left through must be shown open. The rewrite stands in for the original here:
/// every frame of it is checked against the original elsewhere.
///
/// An opening he never uses proves nothing either way, so it is not a
/// failure. Rooms whose shared edge disagrees (open on one side, closed on
/// the other) are counted and reported, not failed: a one-way drop is real.
fn check_map_openings(env: &Env, states: &[Zx]) -> bool {
    use starquake::play::FrameEvent;
    let Some(first) = states.first() else {
        return report("map openings", &[], 0);
    };
    let openings = env.game(first).all_openings();
    let parts: Vec<starquake::map::Parts> = (0..openings.len() as u16)
        .map(|r| env.game(first).room_parts(r))
        .collect();
    let mut inside = 0;
    let mut failures = Vec::new();
    let mut crossings = 0;
    let mut r = Rng(0x3A9);
    for (n, state) in states.iter().enumerate().flat_map(|s| [s; 4]) {
        let mut g = env.game(state);
        let mut input = input_of(state);
        // The part of the room BLOB was first seen in since he entered it.
        let mut part = 0;
        for frame in 0..2000 {
            if frame % 40 == 0 {
                input.kempston = r.byte() & 0x0F;
            }
            let from = g.room;
            let event = g.play_frame(&input);
            if !matches!(event, FrameEvent::Continue | FrameEvent::CoreRoom) {
                break;
            }
            if g.room != from {
                part = 0;
            }
            let (x, y) = (g.entities[0].x(), g.entities[0].y());
            let here = parts
                .get(g.room as usize)
                .map_or(0, |p| p.at((0xBF - y) >> 3, x >> 3));
            if here != 0 && x & 7 == 0 {
                inside += 1;
                if part == 0 {
                    part = here;
                } else if here != part {
                    failures.push((
                        format!("state {n} frame {frame}"),
                        vec![format!(
                            "crossed a wall inside room {} at ({x:#04x}, {y:#04x})",
                            g.room
                        )],
                    ));
                    part = here;
                }
            }
            let o = openings[from as usize];
            let (edge, open) = match g.room.wrapping_sub(from) {
                1 => ("right", o.right),
                0xFFFF => ("left", o.left),
                16 => ("bottom", o.down),
                0xFFF0 => ("top", o.up),
                // Staying put, or a teleporter.
                _ => continue,
            };
            crossings += 1;
            if !open {
                failures.push((
                    format!("state {n} frame {frame}"),
                    vec![format!(
                        "left room {from} through its {edge} edge, shown closed"
                    )],
                ));
            }
            if event == FrameEvent::CoreRoom {
                break;
            }
        }
    }
    let rooms = openings.len();
    let disagree: Vec<usize> = (0..rooms)
        .filter(|&room| {
            let o = openings[room];
            let right = room % 16 < 15 && o.right != openings[room + 1].left;
            let down = room + 16 < rooms && o.down != openings[room + 16].up;
            right || down
        })
        .collect();
    let open: usize = openings
        .iter()
        .map(|o| {
            [o.left, o.right, o.up, o.down]
                .into_iter()
                .filter(|&e| e)
                .count()
        })
        .sum();
    println!(
        "  {open} of {} edges open; {crossings} crossings; {inside} positions inside rooms; rooms that disagree with a neighbour: {disagree:?}",
        rooms * 4
    );
    report("map openings", &failures, crossings)
}

/// A whole main-loop iteration: the original runs with real interrupts
/// from one loop top to the next; the rewrite runs one frame.
fn check_loop(env: &Env, states: &[Zx]) -> bool {
    use starquake::play::FrameEvent;
    let mut failures = Vec::new();
    let mut compared = 0;
    let mut events = std::collections::BTreeMap::<String, usize>::new();
    for (n, state) in states.iter().enumerate() {
        let mut z = state.clone();
        let mut g = env.game(&z);
        let input = input_of(&z);
        let start = z.frame;
        let ok = z.run_until(0xA523, 5);
        let event = g.play_frame(&input);
        *events.entry(format!("{event:?}")).or_default() += 1;
        // Deaths, pauses and screens take many frames; they are not
        // compared here.
        if !ok || z.frame - start > 2 || !matches!(event, FrameEvent::Continue) {
            continue;
        }
        compared += 1;
        let parts = [
            "display", "entities", "status", "printer", "rng", "objects", "restore", "entry",
            "scratch", "pickups", "misc", "spawner",
        ];
        let d = diff(&env.game(&z), &g, &parts);
        if !d.is_empty() {
            failures.push((format!("state {n} (room {})", g.room), d));
        }
    }
    println!("  events: {events:?}");
    report("main loop (A523)", &failures, compared)
}

/// Renders a 4 × 4 montage of rooms, as drawn by the rewrite, starting at
/// the room a new game begins in.
fn render(env: &Env, out: &str) {
    const W: usize = starquake::display::WIDTH;
    const H: usize = starquake::display::HEIGHT;

    let mut z = env.machine();
    let mut misses = zx_runtime::Misses::default();
    let key = |name| zx_runtime::keys::Key::by_name(name).unwrap();
    for f in 0..700 {
        z.release_all_keys();
        if (250..253).contains(&f) {
            z.set_key(key("1"), true);
        }
        if (300..303).contains(&f) {
            z.set_key(key("0"), true);
        }
        z.run_frame(zx_runtime::no_code, &mut misses);
    }
    let start = z.read16(at::ROOM as u16) & 0x1FF;
    println!("game starts in room {start}");

    let mut montage = vec![0u32; W * 4 * H * 4];
    let mut frame = vec![0u32; W * H];
    for i in 0..16 {
        let mut g = env.game(&z);
        g.room = (start + i as u16) % 512;
        g.enter_room_prelude();
        g.restore_mem.fill(0);
        g.restore_ptr = starquake::room::RESTORE_START;
        g.build_room();
        g.display.render(false, &mut frame);
        let (ox, oy) = ((i % 4) * W, (i / 4) * H);
        for y in 0..H {
            montage[(oy + y) * W * 4 + ox..][..W].copy_from_slice(&frame[y * W..][..W]);
        }
    }
    std::fs::write(out, zx_runtime::png::encode(&montage, W * 4, H * 4)).expect("write png");
    println!("wrote {out}");

    // The screens, as the rewrite draws them.
    let dir = std::path::Path::new(out)
        .parent()
        .unwrap_or(std::path::Path::new("."));
    let save = |g: &Game, name: &str| {
        let mut frame = vec![0u32; W * H];
        g.display.render(false, &mut frame);
        let path = dir.join(name);
        std::fs::write(&path, zx_runtime::png::encode(&frame, W, H)).expect("write png");
        println!("wrote {}", path.display());
    };
    let base = new_game_machine(env);
    let mut g = env.game(&base);
    g.intro_screen();
    save(&g, "screen-intro.png");

    let mut g = env.game(&base);
    g.title_draw();
    save(&g, "screen-title.png");

    // The title screen after the player has defined their own keys: the
    // menu should list those, not the ones the tape was saved with.
    let mut g = env.game(&base);
    g.control_method = 5;
    g.udk = *b"QWERT";
    g.title_draw();
    save(&g, "screen-udk.png");

    // The ending, with every piece of the core in place.
    let mut g = env.game(&base);
    g.core_slots = [0, 1, 2, 3, 4, 5, 6, 7, 8];
    g.cores = 5;
    g.cores_left = 0;
    g.ending_screen();
    save(&g, "screen-ending.png");
    let mut g = env.game(&base);
    g.status.score = [0, 4, 2, 1, 5, 0];
    g.cores_left = 4;
    g.frames = 50 * 60 * 7 + 50 * 23;
    g.game_over_screen();
    save(&g, "screen-gameover.png");
    g.core_of_heroes_screen();
    save(&g, "screen-heroes.png");

    // A security door, drawn with the master key so the screen runs through
    // to "access authorised". As in the check, BLOB only reaches a door from
    // some positions around the marker, so sweep them until one triggers.
    'door: for &(room, mx, my) in rooms_with_marker(env, &base, 0).iter().take(4) {
        let mut z = base.clone();
        z.write16(at::ROOM as u16, room);
        z.mem[at::ENTRY_REASON] = 0;
        if !z.call_until(0xA426, Some(0xA523), 20_000_000) {
            continue;
        }
        for dy in [0i32, -1, 1, -4, 4, -8, 8] {
            for dx in [0i32, -2, 2] {
                for input in [1u8, 2] {
                    let mut v = z.clone();
                    v.mem[at::ENTITIES + 5] = mx.wrapping_add(dx as u8);
                    v.mem[at::ENTITIES + 6] = my.wrapping_add(dy as u8);
                    v.mem[at::INVENTORY] = 0x0F;
                    v.mem[at::INVENTORY + 1] = 5;
                    v.release_all_keys();
                    v.kempston = input;
                    let mut g = env.game(&v);
                    let host_input = input_of(&v);
                    g.frame_display();
                    if let starquake::play::FrameEvent::Modal(m) = g.play_logic(&host_input) {
                        g.run_modal(m, &mut starquake::host::NullHost::default());
                        save(&g, "screen-door.png");
                        break 'door;
                    }
                }
            }
        }
    }
}

/// A machine in play, at the top of the main loop just after entering the
/// first room: interrupts on, the stack where the tape's loader put it.
fn play_machine(env: &Env) -> Zx {
    let mut z = new_game_machine(env);
    assert!(z.call_until(0xA426, Some(0xA523), 20_000_000));
    z.t = 0;
    z.int_pending = false;
    z.halted = false;
    z
}

/// Free memory above the ULA's reach, for a `CALL` that stops when it
/// returns. The sound models count from a call in the game's own code, and a
/// stub in the contended 16K would be held up where the game's is not.
const SOUND_STUB: u16 = 0xFFF0;

/// Runs `z` with real 50 Hz interrupts from its current `t` until execution
/// reaches `stop`, recording when the speaker changes, in T-states from the
/// start frame's boundary. Returns the changes and the T-state it stopped.
fn speaker_run(z: &mut Zx, stop: u16) -> Option<(Vec<(u32, bool)>, u32)> {
    const FRAME_T: u32 = starquake::sound::FRAME_T;
    let f0 = z.frame;
    let mut edges = Vec::new();
    let mut level = z.ear;
    // The watch sees the speaker before each instruction, so a change is
    // stamped with the end of the `OUT` that made it. Except when the
    // interrupt was taken straight after: then the stamp would carry the
    // interrupt routine too, and the `OUT` (11 T-states, never contended at
    // the boundary) ended at the previous stamp plus its own length.
    let mut last = (f0, z.t);
    let ok = z.run_until_any_with(&[stop], 100, |m| {
        if m.ear != level {
            level = m.ear;
            let at = if m.frame == last.0 {
                (m.frame - f0) as u32 * FRAME_T + m.t
            } else {
                (last.0 - f0) as u32 * FRAME_T + last.1 + 11
            };
            edges.push((at, level));
        }
        last = (m.frame, m.t);
    });
    ok.then(|| (edges, (z.frame - f0) as u32 * FRAME_T + z.t))
}

/// Speaker changes only: the model reports every `OUT`, and writing the
/// level the speaker already has changes nothing.
fn changes(edges: &[(u32, bool)], mut level: bool) -> Vec<(u32, bool)> {
    let mut out = Vec::new();
    for &(t, l) in edges {
        if l != level {
            level = l;
            out.push((t, l));
        }
    }
    out
}

fn first_difference(orig: &[(u32, bool)], new: &[(u32, bool)]) -> Option<String> {
    let i = orig.iter().zip(new).position(|(a, b)| a != b);
    match i {
        Some(i) => Some(format!(
            "change {i} of {}: original {:?}, rewrite {:?}",
            orig.len(),
            orig[i],
            new[i]
        )),
        None if orig.len() != new.len() => Some(format!(
            "original makes {} changes, rewrite {}",
            orig.len(),
            new.len()
        )),
        None => None,
    }
}

/// Where in a frame the checks start the sound routines: the top of the
/// frame, the first contended line, the middle of the picture where play
/// puts the effects, the last lines, and just before the next interrupt.
const SOUND_STARTS: [u32; 5] = [0, 14_300, 35_150, 57_000, 69_800];

/// The original's run of blocking sound effect `id` from `start`.
fn original_beep(base: &Zx, id: u8, start: u32) -> Option<(Vec<(u32, bool)>, u32)> {
    let mut z = base.clone();
    let stub = SOUND_STUB as usize;
    z.mem[stub..stub + 3].copy_from_slice(&[0xCD, 0xC0, 0xD7]);
    z.a = id;
    z.pc = SOUND_STUB;
    z.t = start;
    z.ear = false;
    speaker_run(&mut z, SOUND_STUB + 3)
}

/// The blocking sound effects (`D7C0`), started at points all through the
/// frame: every speaker change and the length, to the T-state. The ULA holds
/// the effect's stack work up while it draws, and an interrupt breaks into
/// any effect that runs past a boundary, so where it starts changes both.
fn check_beeps(env: &Env) -> bool {
    let base = play_machine(env);
    let mut failures = Vec::new();
    let mut cases = 0;
    for id in 0..starquake::assets::EFFECT_COUNT as u8 {
        for start in SOUND_STARTS {
            cases += 1;
            let case = format!("effect {id:#04x} from {start}");
            let Some((orig, orig_end)) = original_beep(&base, id, start) else {
                failures.push((case, vec!["original did not finish".into()]));
                continue;
            };
            let (edges, end) = starquake::sound::beep(&env.assets.ram, id, start);
            let new = changes(&edges, false);
            let mut d = Vec::new();
            if let Some(diff) = first_difference(&orig, &new) {
                d.push(diff);
            }
            if orig_end != end {
                d.push(format!("ends at {end}, original {orig_end}"));
            }
            if !d.is_empty() {
                failures.push((case, d));
            }
        }
    }
    report("sound effects (D7C0)", &failures, cases)
}

/// The frame tone (`A5BA`): the speaker changes from the start of the tone
/// loop until it sees the next interrupt, for pitches across the range and
/// starts all through the frame.
fn check_tone(env: &Env) -> bool {
    let base = play_machine(env);
    let mut failures = Vec::new();
    let mut cases = 0;
    for half_period in [1u8, 9, 34, 0x7F, 0] {
        for start in SOUND_STARTS {
            cases += 1;
            let case = format!("half period {half_period} from {start}");
            let mut z = base.clone();
            z.c = 0;
            z.e = half_period;
            z.d = z.mem[0x5C78];
            z.push(SOUND_STUB);
            z.pc = 0xA5BA;
            z.t = start;
            z.ear = false;
            let Some((orig, _)) = speaker_run(&mut z, SOUND_STUB) else {
                failures.push((case, vec!["original did not finish".into()]));
                continue;
            };
            let new = changes(&starquake::sound::tone(half_period, start), false);
            if let Some(diff) = first_difference(&orig, &new) {
                failures.push((case, vec![diff]));
            }
        }
    }
    report("tone loop (A5BA)", &failures, cases)
}

/// Least squares: the intercept and one weight per column of `x`.
fn least_squares(x: &[Vec<f64>], y: &[f64]) -> Option<Vec<f64>> {
    let n = x.first()?.len() + 1;
    let mut m = vec![vec![0.0; n + 1]; n];
    for (row, &target) in x.iter().zip(y) {
        let row: Vec<f64> = std::iter::once(1.0).chain(row.iter().copied()).collect();
        for i in 0..n {
            for j in 0..n {
                m[i][j] += row[i] * row[j];
            }
            m[i][n] += row[i] * target;
        }
    }
    for i in 0..n {
        let pivot = (i..n).max_by(|&a, &b| m[a][i].abs().total_cmp(&m[b][i].abs()))?;
        m.swap(i, pivot);
        if m[i][i].abs() < 1e-9 {
            return None;
        }
        let pivot_row = m[i].clone();
        for (r, row) in m.iter_mut().enumerate() {
            if r != i {
                let f = row[i] / pivot_row[i];
                for (cell, p) in row.iter_mut().zip(&pivot_row).skip(i) {
                    *cell -= f * p;
                }
            }
        }
    }
    Some((0..n).map(|i| m[i][n] / m[i][i]).collect())
}

/// The silence at the start of a frame of play: how long the original's
/// work takes before its blocking effects and before its tone, against the
/// rewrite's estimate from what the same frame did (`sound::Work`).
///
/// The two run in lockstep from the start of each frame's work, over play
/// with the joystick moved at random, and the prices are fitted again from
/// what the original took. The check fails if the rewrite's estimate is
/// biased or too loose; the fitted figures are printed so the constants can
/// be brought back in line.
fn check_sound_work(env: &Env) -> bool {
    use starquake::play::FrameEvent;
    use starquake::sound::{FRAME_T, INTERRUPT_T, WORK_BEFORE_EFFECTS_T, WORK_T};
    const ITERATIONS: usize = 3000;
    let mut z = play_machine(env);
    let mut r = Rng(0x50FD);
    let mut tone_rows: Vec<([u32; 5], u32)> = Vec::new();
    let mut effect_rows: Vec<([u32; 5], u32)> = Vec::new();
    for i in 0..ITERATIONS {
        if i % 6 == 0 {
            z.kempston = r.byte() & 0x1F;
        }
        // Past the tone loop and the interrupt, to where the work starts.
        if !z.run_until(0xDF70, 10) {
            break;
        }
        let frame = z.frame;
        let mut g = env.game(&z);
        let room = g.room;
        let input = input_of(&z);
        g.display_work();
        let event = g.play_logic(&input);

        let (mut call, mut entered, mut effects_t) = (None, None, 0u32);
        let at = |m: &Zx| (m.frame - frame) as u32 * FRAME_T + m.t;
        let ok = z.run_until_any_with(&[0xA5BA, 0xA5DC], 10, |m| match m.pc {
            0xD7C0 => {
                call.get_or_insert(at(m) - 17);
                entered = Some(at(m) - 17);
            }
            0xD838 => {
                if let Some(e) = entered.take() {
                    effects_t += at(m) + 10 - e;
                }
            }
            _ => {}
        });
        // Deaths, pauses and room changes do other work altogether.
        if !ok || !matches!(event, FrameEvent::Continue) || g.room != room {
            continue;
        }
        if let (Some(call), Some(before)) = (call, g.work_at_effect) {
            effect_rows.push((before.counts(), call));
        }
        if z.pc == 0xA5BA && (call.is_some() == !g.effects.is_empty()) {
            let tone = at(&z);
            let interrupts = INTERRUPT_T * (tone / FRAME_T);
            tone_rows.push((g.work.counts(), tone - effects_t - interrupts));
        }
    }

    let names = [
        "collision",
        "vertical collision",
        "character",
        "cell",
        "proximity check",
    ];
    let x: Vec<Vec<f64>> = tone_rows
        .iter()
        .map(|(c, _)| c.iter().map(|&n| f64::from(n)).collect())
        .collect();
    let y: Vec<f64> = tone_rows.iter().map(|&(_, t)| f64::from(t)).collect();
    if let Some(fit) = least_squares(&x, &y) {
        let prices: Vec<String> = names
            .iter()
            .zip(&fit[1..])
            .map(|(n, p)| format!("{n} {p:.0}"))
            .collect();
        let mut residual: Vec<f64> = x
            .iter()
            .zip(&y)
            .map(|(row, t)| {
                (t - fit[0] - row.iter().zip(&fit[1..]).map(|(n, p)| n * p).sum::<f64>()).abs()
            })
            .collect();
        residual.sort_by(f64::total_cmp);
        println!(
            "  sound work fitted over {} frames: base {:.0}; {}; 90% within {:.0}",
            tone_rows.len(),
            fit[0],
            prices.join(", "),
            residual[residual.len() * 9 / 10]
        );
    }
    let price = |c: &[u32; 5]| {
        let work = starquake::sound::Work {
            collisions: c[0],
            vertical_collisions: c[1],
            characters: c[2],
            cells: c[3],
            proximity_checks: c[4],
        };
        work.t()
    };
    let mut before: Vec<i64> = effect_rows
        .iter()
        .map(|(c, t)| i64::from(*t) - i64::from(price(c)))
        .collect();
    before.sort_unstable();
    if let Some(m) = before.get(before.len() / 2) {
        println!(
            "  sound work before effects: base median {m} over {} frames",
            before.len()
        );
    }

    let mut failures = Vec::new();
    let mut cases = 0;
    for (name, rows, base, max_p90) in [
        ("tone start", &tone_rows, WORK_T, 3_500u32),
        ("effects start", &effect_rows, WORK_BEFORE_EFFECTS_T, 3_500),
    ] {
        cases += 1;
        let mut errors: Vec<i64> = rows
            .iter()
            .map(|(c, t)| i64::from(*t) - i64::from(base + price(c)))
            .collect();
        if errors.is_empty() {
            failures.push((name.to_string(), vec!["no frames measured".into()]));
            continue;
        }
        errors.sort_unstable();
        let median = errors[errors.len() / 2];
        let mut abs: Vec<u32> = errors.iter().map(|e| e.unsigned_abs() as u32).collect();
        abs.sort_unstable();
        let p90 = abs[abs.len() * 9 / 10];
        println!(
            "  sound work {name}: rewrite off by median {median}, 90% within {p90} ({} frames)",
            errors.len()
        );
        if median.abs() > 400 || p90 > max_p90 {
            failures.push((
                name.to_string(),
                vec![format!("median error {median}, 90th percentile {p90}")],
            ));
        }
    }
    report("sound work (A523)", &failures, cases)
}

/// The blocking sound effects: how long each one takes in the original and
/// in the rewrite, from the top of a frame. The frontend turns that length
/// into whole frames, during which the picture does not change, so an effect
/// that comes out too long shows up as a stall.
fn effects(env: &Env) {
    let base = play_machine(env);
    println!(
        "{:>3}  {:>9}  {:>9}  {:>6}  {:>6}  ",
        "id", "orig", "new", "frames", "edges"
    );
    for id in 0..starquake::assets::EFFECT_COUNT as u8 {
        let orig = original_beep(&base, id, 0);
        let (edges, total) = starquake::sound::beep(&env.assets.ram, id, 0);
        let frames = total as f64 / starquake::sound::FRAME_T as f64;
        let (orig_t, mark) = match orig {
            None => (0, "original did not finish".to_string()),
            Some((_, t)) if t != total => (t, format!("MISMATCH by {}", total as i64 - t as i64)),
            Some((_, t)) => (t, String::new()),
        };
        println!(
            "{id:>3}  {orig_t:>9}  {total:>9}  {frames:>6.2}  {:>6}  {mark}",
            edges.len()
        );
    }
}

/// What the controls actually do with each host key, per control method.
fn keys(env: &Env) {
    use starquake::controls::Input;
    let base = new_game_machine(env);
    // Where the operands live, and what they hold on the tape (what the game
    // itself starts from) versus after the original's own new-game.
    let raw = env.tape.memory();
    let addrs: [(&str, usize); 12] = [
        ("pause port  C55C", 0xC55C),
        ("pause bit   C55F", 0xC55F),
        ("key0 port   C57A", 0xC57A),
        ("key0 bit    C57E", 0xC57E),
        ("key1 port   C585", 0xC585),
        ("key1 bit    C589", 0xC589),
        ("key2 port   C590", 0xC590),
        ("key2 bit    C594", 0xC594),
        ("key3 port   C59B", 0xC59B),
        ("key3 bit    C59F", 0xC59F),
        ("key4 port   C5A6", 0xC5A6),
        ("key4 bit    C5AA", 0xC5AA),
    ];
    println!("operand bytes:        tape       after original new-game");
    for (name, a) in addrs {
        println!("  {name}   {:#04x}       {:#04x}", raw[a], base.mem[a]);
    }
    println!();
    // Keys as the frontend delivers them (see frontend/input.rs).
    let mut space = Input::default();
    space.keys[7] &= !0x01;
    space.kempston = 0x10;
    let mut pkey = Input::default();
    pkey.keys[5] &= !0x01;
    let mut five = Input::default();
    five.keys[3] &= !0x10;
    // The arrow presses "5" as well, as it does on a Spectrum.
    let mut left = Input {
        kempston: 0x02,
        ..Default::default()
    };
    left.keys[3] &= !0x10;
    let cases = [
        ("space", space),
        ("P", pkey),
        ("5", five),
        ("left arrow", left),
    ];

    // The operands persist between games, as they do in the original, so a
    // keyboard method played first leaves its keys in place for Kempston.
    {
        let mut g = env.game(&base);
        g.new_game(5);
        g.new_game(1);
        println!(
            "method 5 then 1: kempston={} pause=(port {:#04x}, bit {})",
            g.controls.kempston, g.controls.pause.0, g.controls.pause.1
        );
        for (name, input) in &cases {
            println!(
                "    {name:<11} pause={:<5} read={:#04x}",
                g.controls.pause_pressed(input),
                g.controls.read(input)
            );
        }
        println!();
    }

    for method in [1u8, 2, 5] {
        let mut g = env.game(&base);
        g.new_game(method);
        println!(
            "method {method}: kempston={} pause=(port {:#04x}, bit {}) keys={:x?}",
            g.controls.kempston, g.controls.pause.0, g.controls.pause.1, g.controls.keys
        );
        for (name, input) in &cases {
            let paused = g.controls.pause_pressed(input);
            let ctl = g.controls.read(input);
            println!(
                "    {name:<11} pause={:<5} read={ctl:#04x}{}{}",
                paused,
                if ctl & 0x10 != 0 { " FIRE" } else { "" },
                if ctl & 0x0F != 0 { " MOVE" } else { "" }
            );
        }
    }
}

/// How fast the original's menu loop actually goes round. The loop has no
/// wait in it, so its speed is however long one redraw of the options takes,
/// and that is what sets how fast the highlight flashes.
/// Turns a second the original's title-menu loop manages, and whether it
/// ended early (in which case the count is short and means nothing).
fn measure_menu_rate(env: &Env) -> (u64, bool) {
    let mut z = new_game_machine(env);
    // Drop straight into the loop, past the title screen and its tune.
    z.push(0);
    z.pc = 0x5FF4;
    z.t = 0;
    let second: u32 = starquake::host::FRAMES_PER_SECOND * starquake::sound::FRAME_T;
    let sp = z.sp;
    let mut turns = 0u64;
    while z.t < second {
        zx_runtime::interp::step(&mut z);
        if z.pc == 0x5FF4 {
            turns += 1;
        }
        // The loop is not supposed to end. If it ever returns or parks, the
        // count would be quietly low, so report that rather than the number.
        if (z.pc == 0 && z.sp == sp) || z.halted {
            return (turns, true);
        }
    }
    (turns, false)
}

fn menu_rate(env: &Env) {
    let (turns, left) = measure_menu_rate(env);
    if left {
        println!("the menu loop left early; the count below is short");
    }
    println!("original menu loop: {turns} turns per second");
    println!(
        "  highlight flips every 2 turns: {:.1} Hz",
        turns as f64 / 2.0
    );
    println!(
        "  the rewrite is paced at {} turns/s: {:.1} Hz",
        starquake::menu::TURNS_PER_SECOND,
        starquake::menu::TURNS_PER_SECOND as f64 / 2.0
    );
}

/// The coverage this run must reach: the counts of the last recorded run,
/// in `coverage-floor.txt` (#121). Coverage only ever grows: a change that
/// drops it drops a check of the original, which is called out, never
/// hidden by lowering the floor to match.
const FLOOR: &str = include_str!("../coverage-floor.txt");

/// Where the program starts: coverage below it is the ROM's, not the game's.
const PROGRAM: u16 = 0x5E00;

/// The original's instructions that ran, and its conditional branches' two
/// directions taken, across the whole run, against the recorded floor.
fn check_coverage_floor() -> bool {
    let (ran, directions) = zx_runtime::coverage::totals(PROGRAM);
    let floor = |name: &str| {
        FLOOR
            .lines()
            .find_map(|l| l.strip_prefix(name)?.trim().parse::<usize>().ok())
            .unwrap_or(0)
    };
    let (min_ran, min_dirs) = (floor("instructions run"), floor("branch directions taken"));
    println!(
        "coverage: {ran} instructions of the original run, {directions} branch directions taken (floor {min_ran}, {min_dirs})"
    );
    let mut failures = Vec::new();
    if ran < min_ran || directions < min_dirs {
        failures.push((
            "coverage".into(),
            vec![format!(
                "below the floor: {ran} of {min_ran} instructions, {directions} of {min_dirs} directions"
            )],
        ));
    }
    report("coverage of the original (#121)", &failures, 1)
}

/// `sq-verify coverage`: the original's code as `zx-recomp` finds it from
/// the tape, against what this run's checks ran, block by block, into
/// `.scratch/coverage.txt` (#121). It names addresses only; the listing to
/// read a gap against is `zx-recomp --listing`. Never committed.
fn coverage_report(analysis: &zx_recomp::analysis::Analysis) {
    use zx_core::Instr::{Call, Djnz, Jp, Jr, Ret};
    use zx_runtime::coverage::{NOT_TAKEN, RAN, TAKEN};
    let map = zx_runtime::coverage::map();
    let mut found = 0;
    let mut ran = 0;
    let (mut branches, mut both, mut one) = (0, 0, 0);
    let mut unrun = Vec::new();
    let mut half = Vec::new();
    let mut seen = std::collections::HashSet::new();
    for block in analysis.blocks.iter().filter(|b| b.start >= PROGRAM) {
        let mut block_ran = 0;
        for i in &block.instrs {
            if !seen.insert(i.addr) {
                continue;
            }
            found += 1;
            let m = map[usize::from(i.addr)];
            if m & RAN != 0 {
                ran += 1;
                block_ran += 1;
            }
            if matches!(
                i.d.instr,
                Jp(Some(_), _) | Jr(Some(_), _) | Call(Some(_), _) | Ret(Some(_)) | Djnz(_)
            ) {
                branches += 1;
                match (m & TAKEN != 0, m & NOT_TAKEN != 0) {
                    (true, true) => both += 1,
                    (false, false) => {}
                    _ => {
                        one += 1;
                        half.push((i.addr, if m & TAKEN != 0 { "taken" } else { "not taken" }));
                    }
                }
            }
        }
        if block_ran == 0 && !block.instrs.is_empty() {
            unrun.push((block.start, block.instrs.len()));
        }
    }
    unrun.sort_by(|a, b| b.1.cmp(&a.1).then(a.0.cmp(&b.0)));
    let mut out = format!(
        "The original's code from {PROGRAM:04x} up, as zx-recomp finds it from the tape,\n\
         against what sq-verify's checks ran (#121). The analysis also decodes some\n\
         data as code (tables and graphics reached through a misread jump), so the\n\
         totals below overstate the code there is, and a block never run may be data.\n\n\
         instructions: {ran} of {found} run ({:.1}%)\n\
         conditional branches: {branches}; both ways {both}, one way only {one}, never {}\n\n\
         blocks never run, largest first (start, instructions):\n",
        100.0 * ran as f64 / found.max(1) as f64,
        branches - both - one
    );
    for (start, n) in &unrun {
        out.push_str(&format!("  {start:04x}  {n}\n"));
    }
    out.push_str("\nbranches taken one way only (address, the way taken):\n");
    for (a, way) in &half {
        out.push_str(&format!("  {a:04x}  {way}\n"));
    }
    let _ = std::fs::create_dir_all(".scratch");
    match std::fs::write(".scratch/coverage.txt", &out) {
        Ok(()) => println!(
            "coverage report: {ran} of {found} instructions run, {both} of {branches} branches both ways; .scratch/coverage.txt"
        ),
        Err(e) => println!("coverage report not written: {e}"),
    }
}

/// The original's code found from the tape by `zx-recomp`, for the report.
/// Its trace runs the original too, so this comes before recording starts.
fn analyse(dir: &std::path::Path) -> zx_recomp::analysis::Analysis {
    let cfg = zx_recomp::Config::parse(include_str!("../../re/starquake.toml"))
        .expect("tools/re/starquake.toml");
    let inputs = zx_recomp::Inputs::load(&cfg, dir).expect("the tape and the ROM");
    let traced = zx_recomp::tracer::run(&cfg.trace, &inputs, |_, _| {}).expect("trace");
    zx_recomp::analysis::analyze(
        &cfg,
        &inputs.memory(),
        inputs.start.pc,
        inputs.rom.is_some(),
        &traced.trace,
        &[],
    )
}

fn main() {
    let mut args = std::env::args().skip(1).collect::<Vec<_>>();
    let dir = match args.iter().position(|a| a == "--assets") {
        Some(i) => {
            if i + 1 >= args.len() {
                eprintln!("--assets needs a directory");
                std::process::exit(2);
            }
            let d = PathBuf::from(args.remove(i + 1));
            args.remove(i);
            d
        }
        None => PathBuf::from("assets"),
    };
    let coverage = args.first().map(String::as_str) == Some("coverage");
    let analysis = coverage.then(|| analyse(&dir));
    // Everything from here, the program's start-up included, counts (#121).
    zx_runtime::coverage::start();
    let tape_bytes = std::fs::read(dir.join("starquake.tap")).expect("read starquake.tap");
    assert!(
        starquake::assets::is_supported_tape(&tape_bytes),
        "starquake.tap is not the supported tape (SHA-1 {})",
        starquake::assets::TAPE_SHA1
    );
    let rom = std::fs::read(dir.join("48.rom")).expect("read 48.rom");
    let tape = zx_core::tape::load_tap(&tape_bytes).expect("parse starquake.tap");
    let mut start = Zx::new(
        &zx_core::MachineState::from_tape(&tape, at::ENTRY_PC, at::ENTRY_SP),
        Some(&rom),
    );
    // The program's own start-up runs on the real ROM, up to its menu, and
    // the menu sets its font (`CHARS`), draws the title screen and plays the
    // title tune with interrupts off. Every check starts once the tune has
    // played out and the menu waits for a key, interrupts on.
    assert!(
        start.run_until(MENU, 500),
        "the tape's program did not reach its menu at {MENU:04x}"
    );
    let mut misses = zx_runtime::Misses::default();
    let mut frames = 0;
    while !start.iff1 {
        assert!(frames < 2_000, "the title tune did not end");
        start.run_frame(zx_runtime::no_code, &mut misses);
        frames += 1;
    }
    for _ in 0..10 {
        start.run_frame(zx_runtime::no_code, &mut misses);
    }
    assert_eq!(
        u16::from_le_bytes([start.mem[0x5C36], start.mem[0x5C37]]),
        0xACD4,
        "the menu set the game's font"
    );
    let assets = Rc::new(Assets::from_memory(&tape.memory()));
    let env = Env {
        start,
        assets,
        tape,
    };

    if args.first().map(String::as_str) == Some("sound") {
        let ok = check_beeps(&env) & check_tone(&env) & check_sound_work(&env);
        std::process::exit(i32::from(!ok));
    }
    if args.first().map(String::as_str) == Some("effects") {
        effects(&env);
        return;
    }

    if args.first().map(String::as_str) == Some("menu") {
        menu_rate(&env);
        return;
    }

    if args.first().map(String::as_str) == Some("keys") {
        keys(&env);
        return;
    }

    if args.first().map(String::as_str) == Some("probe") {
        probe(&env);
        return;
    }

    if args.first().map(String::as_str) == Some("render") {
        render(&env, args.get(1).map_or("rooms.png", String::as_str));
        return;
    }

    // Panic messages are silenced so an expected panic inside a check does
    // not spew; `guarded` reports one as a failed check instead. `SQ_PANIC=1`
    // shows them, for when a panic is the thing being investigated.
    if std::env::var_os("SQ_PANIC").is_none() {
        std::panic::set_hook(Box::new(|_| {}));
    }
    let mut ok = true;
    ok &= guarded("room tiles", || check_rooms(&env));
    ok &= guarded("room prelude (panel)", || check_room_prelude(&env));
    ok &= guarded("room build with pickups (new game)", || {
        check_room_build(&env)
    });
    ok &= guarded("room entry with enemies (room chains)", || {
        check_room_entry(&env)
    });
    ok &= guarded("new game (629D)", || check_new_game(&env));
    ok &= guarded("gamepad in every control method", || {
        check_gamepad_methods(&env)
    });
    ok &= guarded("menu (5E81)", || check_menu(&env));
    ok &= guarded("screens", || check_screens(&env));
    ok &= guarded("core room (A6C1)", || check_core_room(&env));
    ok &= guarded("music (D9DE)", || check_music(&env));
    ok &= guarded("sound effects (D7C0)", || check_beeps(&env));
    ok &= guarded("tone loop (A5BA)", || check_tone(&env));
    ok &= guarded("sound work (A523)", || check_sound_work(&env));

    let states = guarded_states("gameplay states", || gameplay_states(&env, 150, 7));
    println!("gameplay states: {}", states.len());
    let frame_routines: [FrameRoutine; 5] = [
        ("sprites (DF70)", 0xDF70, Game::draw_sprites),
        ("sprite colours (D8B1)", 0xD8B1, Game::colour_sprites),
        ("platforms (DBEC)", 0xDBEC, Game::tick_platforms),
        ("sparkles (DCE6)", 0xDCE6, Game::tick_sparkles),
        ("force fields (A66C)", 0xA66C, Game::tick_force_fields),
    ];
    for (name, addr, f) in frame_routines {
        ok &= guarded(name, || check_frame_routine(&env, &states, name, addr, f));
    }
    ok &= guarded("enemies (A01B)", || check_enemies(&env, &states));
    ok &= guarded("BLOB (C5BD)", || check_blob(&env, &states));
    ok &= guarded("main loop (A523)", || check_loop(&env, &states));
    ok &= guarded("death sequence (C350)", || check_death(&env, &states));
    ok &= guarded("game over screen (6730)", || check_game_over(&env, &states));
    ok &= guarded("lift boarded walking right (#117)", || check_lift(&env));
    ok &= guarded("pictures under blocking effects (#116)", || {
        check_effect_pictures(&env)
    });
    ok &= guarded("ending a paused game (#125)", || {
        check_end_while_paused(&env, &states)
    });
    ok &= guarded("gamepad types no initials (#123)", || {
        check_pad_types_nothing(&env, &states)
    });
    ok &= guarded("security doors (D5FD)", || check_security_doors(&env));

    let tour = guarded_states("room tour states", || room_tour_states(&env, 120));
    println!("room tour states: {}", tour.len());
    ok &= guarded("tour: sprite colours", || {
        check_frame_routine(
            &env,
            &tour,
            "tour: sprite colours",
            0xD8B1,
            Game::colour_sprites,
        )
    });
    ok &= guarded("tour: force fields", || {
        check_frame_routine(
            &env,
            &tour,
            "tour: force fields",
            0xA66C,
            Game::tick_force_fields,
        )
    });
    ok &= guarded("enemies (A01B)", || check_enemies(&env, &tour));
    ok &= guarded("BLOB (C5BD)", || check_blob(&env, &tour));
    ok &= guarded("main loop (A523)", || check_loop(&env, &tour));
    ok &= guarded("map openings", || {
        let both: Vec<Zx> = states.iter().chain(&tour).cloned().collect();
        check_map_openings(&env, &both)
    });
    ok &= guarded("coverage of the original (#121)", check_coverage_floor);
    if let Some(analysis) = &analysis {
        coverage_report(analysis);
    }
    if !ok {
        std::process::exit(1);
    }
}
