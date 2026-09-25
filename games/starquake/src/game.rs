//! The complete game state.

use std::rc::Rc;

use crate::assets::Assets;
use crate::controls::{Controls, Input};
use crate::display::{self, Display};
use crate::entities::{Entity, SLOTS, Spawner};
use crate::hud::Status;
use crate::layout as at;
use crate::pickups::{Bonus, ITEM_COUNT, Item, RoomSet};
use crate::printer::Printer;
use crate::rng::Rng;
use crate::room::{FORCE_FIELDS_LEN, Marker, RoomObjects, SPARKLE_TABLE_LEN};

/// The parts of the program, as a frontend sees them.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Scene {
    /// The picture the tape showed while it loaded.
    Loading,
    /// The title screen and its menu, and a new game's intro.
    Menu,
    /// A game being played, with the deaths along the way.
    Play,
    /// The end of a game: the scores, entering initials, the high-score table.
    GameOver,
}

/// A teleporter whose booth has been entered: the room it is in and its
/// code.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct SeenTeleporter {
    pub room: u16,
    pub code: [u8; 5],
}

#[derive(Clone)]
pub struct Game {
    pub assets: Rc<Assets>,
    pub display: Display,
    pub printer: Printer,
    pub rng: Rng,
    /// Attribute used for placeholder tile cells; carries over from
    /// whatever set it last.
    pub colour: u8,
    /// The current room's four colours.
    pub room_colours: [u8; 4],
    /// Bright cells whose colours are re-applied after sprites are drawn:
    /// the list's memory (see [`crate::room::RESTORE_START`]) and its end
    /// pointer (0 while the panel is drawn).
    pub restore_mem: Vec<u8>,
    pub restore_ptr: u16,
    pub controls: Controls,
    /// Alternates between the two footstep sounds.
    pub footstep_sound: u8,
    pub room: u16,
    pub objects: RoomObjects,
    /// Teleporter table: (room low byte, room high bit in bit 7) per entry.
    pub teleporters: Vec<(u8, u8)>,
    pub status: Status,
    pub entities: [Entity; SLOTS],
    /// Enemies of the room before the last, parked for going back.
    pub enemy_cache: [[u8; 21]; 4],
    pub spawner: Spawner,
    /// BLOB's platforms: 12 raw 4-byte records (col | stage << 5,
    /// row, -, timer).
    pub platforms: Vec<u8>,
    /// Platform animated next.
    pub platform_cursor: u8,
    /// 50 Hz frame counter (24 bits, like the original's).
    pub frames: u32,
    /// The last game's rooms-visited score, the score as digits, and the
    /// eight high scores (3 initials, 6 digits, rooms visited).
    pub adventure: u8,
    pub score_digits: [u8; 6],
    pub high_scores: Vec<u8>,
    /// Why the current room was entered (see [`crate::entry::reason`]).
    pub entry_reason: u8,
    /// Where BLOB entered the current room, and in which state.
    pub saved_position: (u8, u8),
    pub saved_state: u8,
    /// Core pieces delivered.
    pub cores: u8,
    /// Collision scratch (last flags and attribute address).
    pub collision: [u8; 3],
    /// Sound tick state (two effect channels).
    pub sound: [u8; 8],
    /// Enemy update loop counters.
    pub enemy_cursor: [u8; 2],
    /// Blocking sound effects requested since the frontend last looked.
    pub effects: Vec<u8>,
    /// Tone to play for the rest of this frame (see [`Game::sound_tick`]).
    pub tone: Option<u8>,
    /// The teleporters whose booths have been entered this game, in the
    /// order they were seen, for a frontend that shows their codes (#50)
    /// and where they are (#2). Not part of the original's state: nothing
    /// in the game reads it.
    pub teleporters_seen: Vec<SeenTeleporter>,
    /// Which part of the program is running, for a frontend that shows
    /// something beside it. Not part of the original's state: nothing in
    /// the game reads it.
    pub scene: Scene,
    /// Whether this frame boundary is the play loop's. The original spends
    /// the start of such a frame on the frame's work, in silence, before
    /// the sound (see [`Game::frame_sound`]).
    pub play_work: bool,
    /// Which of up's and down's meanings the frame's gamepad press carries
    /// (#112). Not part of the original's state.
    pub pad: crate::controls::PadMeaning,
    /// What the frame's work has done so far, which is how long it took the
    /// original (see [`crate::sound::Work`]).
    pub work: crate::sound::Work,
    /// [`Game::work`] when the frame's first blocking effect was requested.
    pub work_at_effect: Option<crate::sound::Work>,
    /// Speaker changes of a tune playing over this frame, as (T-states into
    /// the frame, level). Empty unless a tune is playing.
    pub music: Vec<(u32, bool)>,
    /// Input for the current frame (from the host).
    pub input: crate::controls::Input,
    /// The nine core slots: piece graphics, bit 7 set while missing.
    pub core_slots: [u8; 9],
    pub cores_left: u8,
    pub var_d2bf: u8,
    pub var_d2e9: u8,
    pub var_d2ea: u8,
    /// Player-defined keys (key names: left, right, down, up, fire) and the
    /// pause key, which every control method uses.
    pub udk: [u8; 5],
    pub udk_pause: u8,
    /// The control method the menu selected (1 Kempston … 5 own keys).
    pub control_method: u8,
    /// While set, text is printed with the title screen's own UDGs.
    pub title_udg: bool,
    /// Death kind; the core grid's column shares this byte in the original.
    pub death_kind: u8,
    /// Where the 3 x 3 core grid is drawn, as (column, row). The original
    /// keeps the column in the same byte as the death reason above; nothing
    /// here depends on that, and the two are never in use at once.
    pub core_grid: (u8, u8),
    pub death_ink: u8,
    /// Ink last chosen for screen texts, and the flashing phase.
    pub screen_ink: u8,
    pub flash_phase: u8,
    /// The current code check: position (col, row), length, and
    /// (graphic, attribute) per code item.
    pub code_pos: (u8, u8),
    pub code_len: u8,
    pub code: [u8; 6],
    /// Pyramid trade: four offers and the item given.
    pub offers: [u8; 5],
    /// Letters typed at a teleporter booth.
    pub typed_code: [u8; 5],
    pub items: Vec<Item>,
    /// Rooms that may still offer a bonus pickup.
    pub bonus_rooms: RoomSet,
    /// Rooms not visited yet (first visit scores).
    pub unvisited_rooms: RoomSet,
    /// Per-game random seed.
    pub seed: u16,
    pub bonus: Bonus,
    pub pickups_in_room: u8,
    /// Spawn point used for the last placed item; the bonus avoids it.
    pub last_spawn_index: u8,
}

impl Game {
    /// Reads the game state out of a memory image of the original program:
    /// the player's snapshot at startup, or the reference machine when
    /// verifying.
    /// Builds the game's state from the memory the original starts with.
    ///
    /// # Panics
    ///
    /// If `mem` is shorter than a 48K machine's memory.
    pub fn from_memory(assets: Rc<Assets>, mem: &[u8]) -> Game {
        let word = |a: usize| mem[a] as u16 | (mem[a + 1] as u16) << 8;
        let bytes = |a: usize, n: usize| &mem[a..a + n];

        Game {
            display: Display {
                mem: bytes(0x4000, display::LEN).try_into().unwrap(),
                border: 0,
            },
            printer: Printer::at(
                24u8.wrapping_sub(mem[at::S_POSN + 1]),
                33u8.wrapping_sub(mem[at::S_POSN]),
                mem[at::ATTR_T],
                mem[at::MASK_T],
                mem[at::P_FLAG],
            ),
            rng: Rng {
                a: word(at::RNG),
                b: word(at::RNG + 2),
                c: word(at::RNG + 4),
                b_countdown: mem[at::RNG_COUNTDOWNS],
                c_countdown: mem[at::RNG_COUNTDOWNS + 1],
            },
            colour: mem[at::COLOUR],
            room_colours: bytes(at::ROOM_COLOURS, 4).try_into().unwrap(),
            restore_mem: bytes(at::RESTORE_LIST, crate::room::RESTORE_LEN).to_vec(),
            restore_ptr: word(at::RESTORE_PTR),
            controls: Controls::from_memory(mem),
            footstep_sound: mem[at::FOOTSTEP],
            room: word(at::ROOM),
            objects: objects_from_memory(mem),
            teleporters: (0..8)
                .map(|i| {
                    (
                        mem[at::TELEPORTERS + i * 2],
                        mem[at::TELEPORTERS + i * 2 + 1],
                    )
                })
                .collect(),
            status: Status {
                score: bytes(at::SCORE, 6).try_into().unwrap(),
                pending: bytes(at::SCORE_PENDING, 6).try_into().unwrap(),
                lives: mem[at::LIVES],
                bars: bytes(at::BARS, 3).try_into().unwrap(),
                inventory: std::array::from_fn(|i| {
                    (mem[at::INVENTORY + i * 2], mem[at::INVENTORY + i * 2 + 1])
                }),
                incoming: (mem[at::INVENTORY - 2], mem[at::INVENTORY - 1]),
                outgoing: (mem[at::INVENTORY + 8], mem[at::INVENTORY + 9]),
            },
            entities: std::array::from_fn(|k| {
                Entity(bytes(at::ENTITIES + k * 32, 32).try_into().unwrap())
            }),
            enemy_cache: std::array::from_fn(|k| {
                bytes(at::ENEMY_CACHE + k * 21, 21).try_into().unwrap()
            }),
            spawner: Spawner {
                timer: mem[at::SPAWN_TIMER],
                last_room: word(at::SPAWN_LAST_ROOM),
                count: mem[at::SPAWN_COUNT],
                room_before: word(at::SPAWN_ROOM_BEFORE),
                count_before: mem[at::SPAWN_COUNT_BEFORE],
                masks: bytes(at::SPAWN_MASKS, 4).try_into().unwrap(),
                params: bytes(at::SPAWN_PARAMS, 4).try_into().unwrap(),
                seed: mem[at::SPAWN_SEED],
                tries: mem[at::SPAWN_TRIES],
            },
            platforms: bytes(at::PLATFORMS, at::PLATFORMS_LEN).to_vec(),
            platform_cursor: mem[at::PLATFORM_CURSOR],
            frames: word(at::FRAMES) as u32 | (mem[at::FRAMES + 2] as u32) << 16,
            adventure: mem[at::ADVENTURE],
            score_digits: bytes(at::SCORE_DIGITS, 6).try_into().unwrap(),
            high_scores: bytes(at::HIGH_SCORES, 80).to_vec(),
            entry_reason: mem[at::ENTRY_REASON],
            saved_position: (mem[at::SAVED_POSITION], mem[at::SAVED_POSITION + 1]),
            saved_state: mem[at::SAVED_STATE],
            cores: mem[at::CORES],
            collision: bytes(at::COLLISION, 3).try_into().unwrap(),
            sound: bytes(at::SOUND, 8).try_into().unwrap(),
            enemy_cursor: bytes(at::ENEMY_CURSOR, 2).try_into().unwrap(),
            effects: Vec::new(),
            tone: None,
            scene: Scene::Loading,
            teleporters_seen: Vec::new(),
            play_work: false,
            pad: crate::controls::PadMeaning::default(),
            work: crate::sound::Work::default(),
            work_at_effect: None,
            music: Vec::new(),
            input: Input::default(),
            core_slots: bytes(at::CORE_SLOTS, 9).try_into().unwrap(),
            cores_left: mem[at::CORES_LEFT],
            var_d2bf: mem[at::VAR_D2BF],
            var_d2e9: mem[at::VAR_D2E9],
            var_d2ea: mem[at::VAR_D2E9 + 1],
            udk: bytes(at::UDK, 5).try_into().unwrap(),
            udk_pause: mem[at::UDK_PAUSE],
            control_method: mem[at::CONTROL_METHOD],
            title_udg: false,
            death_kind: mem[at::DEATH_KIND],
            core_grid: (mem[at::DEATH_KIND], mem[at::DEATH_KIND + 1]),
            death_ink: mem[at::DEATH_INK],
            screen_ink: mem[at::SCREEN_INK],
            flash_phase: mem[at::FLASH_PHASE],
            code_pos: (mem[at::CODE_POS], mem[at::CODE_POS + 1]),
            code_len: mem[at::CODE_POS + 2],
            code: bytes(at::CODE_POS + 3, 6).try_into().unwrap(),
            offers: bytes(at::OFFERS, 5).try_into().unwrap(),
            typed_code: bytes(at::TYPED_CODE, 5).try_into().unwrap(),
            items: (0..ITEM_COUNT)
                .map(|k| Item(bytes(at::ITEMS + k * 4, 4).try_into().unwrap()))
                .collect(),
            bonus_rooms: RoomSet(bytes(at::BONUS_ROOMS, 64).try_into().unwrap()),
            unvisited_rooms: RoomSet(bytes(at::UNVISITED_ROOMS, 64).try_into().unwrap()),
            seed: word(at::SEED),
            bonus: Bonus {
                col: mem[at::BONUS],
                row: mem[at::BONUS + 1],
                graphic: mem[at::BONUS + 2],
                attr: mem[at::BONUS + 3],
            },
            pickups_in_room: mem[at::PICKUPS_IN_ROOM],
            last_spawn_index: mem[at::LAST_SPAWN_INDEX],
            assets,
        }
    }
}

fn objects_from_memory(mem: &[u8]) -> RoomObjects {
    use at::objects::{
        KIND12, MARKERS, MARKERS_END, SPARKLE_CURSOR, SPARKLES, SPAWN, SPAWN_COUNT, TELEPORT_ENTRY,
        TELEPORT_POS, TYPE7, TYPE7_COUNT, TYPE8, TYPE8_COUNT,
    };
    let word = |a: usize| mem[a] as usize | (mem[a + 1] as usize) << 8;
    let pairs = |start: usize, count: usize| -> Vec<(u8, u8)> {
        (0..count)
            .map(|i| (mem[start + i * 2], mem[start + i * 2 + 1]))
            .collect()
    };
    let teleport_entry = word(TELEPORT_ENTRY);
    let kind12 = word(KIND12);
    RoomObjects {
        sparkle_table: mem[SPARKLES..SPARKLES + SPARKLE_TABLE_LEN].to_vec(),
        sparkle_cursor: word(SPARKLE_CURSOR) as u16,
        force_fields: mem[TYPE7..TYPE7 + FORCE_FIELDS_LEN].to_vec(),
        force_field_cursor: mem[TYPE7_COUNT],
        type8: pairs(TYPE8, mem[TYPE8_COUNT] as usize),
        spawn_points: pairs(SPAWN, mem[SPAWN_COUNT] as usize),
        markers: (MARKERS..word(MARKERS_END).max(MARKERS))
            .step_by(3)
            .map(|a| Marker {
                x: mem[a],
                y: mem[a + 1],
                kind: mem[a + 2],
            })
            .collect(),
        teleport: (teleport_entry != 0).then(|| {
            let pos = (mem[TELEPORT_POS], mem[TELEPORT_POS + 1]);
            (pos, (teleport_entry - 1 - at::TELEPORTERS) / 2)
        }),
        kind12: (kind12 != 0).then_some((kind12 as u8, (kind12 >> 8) as u8)),
    }
}
