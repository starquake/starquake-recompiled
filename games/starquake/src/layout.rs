//! Where the original program keeps its state.
//!
//! The initial game state is taken from the player's snapshot, and the
//! verification tool reads the reference machine through the same map, so
//! the addresses live in one place.

pub const ROOM: usize = 0xD2C8;
pub const RNG: usize = 0xDAC0;
pub const RNG_COUNTDOWNS: usize = 0xDB19;
pub const COLOUR: usize = 0xEA63;
pub const ROOM_COLOURS: usize = 0xA7F8;
pub const RESTORE_PTR: usize = 0xEA60;
pub const RESTORE_LIST: usize = 0x5B20;
pub const TELEPORTERS: usize = 0x95F0;

pub const SCORE: usize = 0xD413;
pub const SCORE_PENDING: usize = 0xD419;
pub const LIVES: usize = 0xD2CC;
pub const BARS: usize = 0xD2CD;
pub const INVENTORY: usize = 0xD2D2;

/// Six 32-byte entity slots; slot 0 is BLOB.
pub const ENTITIES: usize = 0xDD18;
pub const ENEMY_CACHE: usize = 0x959C;
pub const SPAWN_TIMER: usize = 0x9C40;
pub const SPAWN_LAST_ROOM: usize = 0x9C41;
pub const SPAWN_COUNT: usize = 0x9C43;
pub const SPAWN_ROOM_BEFORE: usize = 0x9C44;
pub const SPAWN_COUNT_BEFORE: usize = 0x9C46;
/// Enemy roll state, kept inside the spawner's code.
pub const SPAWN_SEED: usize = 0x9DB9;
pub const SPAWN_MASKS: usize = 0x9DBA;
pub const SPAWN_PARAMS: usize = 0x9DBE;
pub const SPAWN_TRIES: usize = 0x9F04;
pub const PLATFORMS: usize = 0xDBBB;
pub const PLATFORMS_LEN: usize = 0x31;
pub const PLATFORM_CURSOR: usize = 0xDBBA;
/// Collision scratch: flags, last attribute address.
pub const COLLISION: usize = 0xD3BE;
/// Sound effect state for the frame tick (two channels).
pub const SOUND: usize = 0xA41B;
/// Enemy update loop counters (inside its code).
pub const ENEMY_CURSOR: usize = 0xA019;
/// Footstep sound number (inside BLOB's code).
pub const FOOTSTEP: usize = 0xC6FF;
pub const CORE_SLOTS: usize = 0xD2DE;
pub const CORES_LEFT: usize = 0xD2E7;
pub const VAR_D2BF: usize = 0xD2BF;
pub const VAR_D2E9: usize = 0xD2E9;
/// The control method the menu last selected (1 Kempston … 5 own keys).
pub const CONTROL_METHOD: usize = 0x5E58;
/// Player-defined keys (5 key names), then the pause key (all methods use
/// it, but only the define-keys screen sets it).
pub const UDK: usize = 0x5E6B;
pub const UDK_PAUSE: usize = 0x5E70;
/// Death kind and the ink the panel flashes with (inside the death code).
pub const DEATH_KIND: usize = 0xC4A9;
pub const DEATH_INK: usize = 0xC3A2;
/// Screen text ink and flash phase.
pub const SCREEN_INK: usize = 0xD589;
pub const FLASH_PHASE: usize = 0xD59F;
/// Code check: col, row, length, then (graphic, attribute) × 3.
pub const CODE_POS: usize = 0xD5F4;
pub const OFFERS: usize = 0xCCEA;
pub const TYPED_CODE: usize = 0xD031;
/// End of game: rooms-visited score, the score as digits, the high scores.
pub const ADVENTURE: usize = 0x654A;
pub const SCORE_DIGITS: usize = 0x67EA;
pub const HIGH_SCORES: usize = 0x64FA;
/// ROM frame counter (system variable FRAMES).
pub const FRAMES: usize = 0x5C78;
pub const ENTRY_REASON: usize = 0xD2C4;
pub const SAVED_STATE: usize = 0xD2C5;
pub const SAVED_POSITION: usize = 0xD2DC;
pub const CORES: usize = 0xD2E8;

pub const ITEMS: usize = 0x94E8;
pub const BONUS_ROOMS: usize = 0xA350;
pub const UNVISITED_ROOMS: usize = 0xA390;
pub const SEED: usize = 0xD2C6;
pub const BONUS: usize = 0xD2C0;
pub const PICKUPS_IN_ROOM: usize = 0xD2BE;
/// Kept inside the code of the pickup routine.
pub const LAST_SPAWN_INDEX: usize = 0xAA9F;

/// ROM print state (system variables).
pub const S_POSN: usize = 0x5C88;
pub const ATTR_T: usize = 0x5C8F;
pub const MASK_T: usize = 0x5C90;
pub const P_FLAG: usize = 0x5C91;

/// Room objects (the block cleared on room entry starts at `0x9600`).
pub mod objects {
    pub const TELEPORT_POS: usize = 0x9602;
    pub const TELEPORT_ENTRY: usize = 0x9605;
    pub const TYPE8_COUNT: usize = 0x9620;
    pub const TYPE8: usize = 0x9621;
    pub const TYPE7_COUNT: usize = 0x9634;
    pub const TYPE7: usize = 0x9635;
    pub const SPARKLE_CURSOR: usize = 0x9664;
    pub const SPARKLES: usize = 0x9666;
    pub const SPAWN_COUNT: usize = 0x96CA;
    pub const SPAWN: usize = 0x96CB;
    pub const MARKERS_END: usize = 0x96FA;
    pub const MARKERS: usize = 0x96FC;
    pub const KIND12: usize = 0xD2CA;
}
