//! Game data, read from the player's own copy of Starquake.
//!
//! Nothing here is original data: this module only knows *where* in the
//! original program each table lives, and parses it at startup.

use std::path::Path;

use zx_core::sha1::sha1_hex;

/// SHA-1 of the supported files: the `.tap` tape the game loads from, and
/// the `.z80` snapshot the verification tool uses.
pub const TAPE_SHA1: &str = "65450d6f33692c2c2868c0b497037f2cfd0ef3bd";
pub const SNAPSHOT_SHA1: &str = "8cf0722b752f7fe1651734b32a240e714525e480";

/// Reads the player's own copy of the game — a `.tap` tape or a `.z80`
/// snapshot — into a memory image, along with the picture a tape painted
/// while it loaded.
/// Reads the player's own copy of the game, from a `.tap` tape or a `.z80`
/// snapshot, and returns its memory and the tape's loading picture.
///
/// # Errors
///
/// If the file cannot be read, has an extension that is neither, or does
/// not parse as the format its extension claims.
pub fn read_game(path: &Path) -> Result<(Vec<u8>, Option<Vec<u8>>), String> {
    let bytes = std::fs::read(path).map_err(|e| {
        format!(
            "cannot read {}: {e}\nStarquake needs your own copy of the original game; \
             see assets/README.md.",
            path.display()
        )
    })?;
    let hash = sha1_hex(&bytes);
    let want = |expected: &str| -> Result<(), String> {
        if hash == expected {
            Ok(())
        } else {
            Err(format!(
                "{} has SHA-1 {hash}; this version supports only {expected}",
                path.display()
            ))
        }
    };
    if path
        .extension()
        .is_some_and(|e| e.eq_ignore_ascii_case("tap"))
    {
        want(TAPE_SHA1)?;
        let tape = zx_core::tape::load_tap(&bytes)?;
        Ok((tape.memory(), tape.loading_screen))
    } else {
        want(SNAPSHOT_SHA1)?;
        Ok((zx_core::snapshot::load_z80(&bytes)?.memory(), None))
    }
}

/// Locations of the tables in the original program's memory.
mod addr {
    pub const ROOMS: usize = 0x7530;
    pub const TILE_INFO: usize = 0x9740;
    pub const BIG_BLOCKS: usize = 0x9840;
    pub const TILE_POINTERS: usize = 0xEB23;
    /// Font pointer (glyph for character `c` at `FONT + 8·c`).
    pub const FONT: usize = 0xACD4;
    pub const UDG: usize = 0xAEAC;
    /// UDGs used only by the title line.
    pub const TITLE_UDG: usize = 0x5E71;
    /// 2 × 2 character graphics, 32 bytes each.
    pub const GRAPHICS32: usize = 0x9088;
}

fn glyphs<const N: usize>(mem: &[u8], start: usize) -> [[u8; 8]; N] {
    std::array::from_fn(|i| mem[start + i * 8..][..8].try_into().unwrap())
}

pub const ROOM_COUNT: usize = 512;

/// Blocking sound effects the game can ask for. Past this the parameter
/// table holds whatever follows it, and the effect never ends.
pub const EFFECT_COUNT: usize = 0x16;

/// A tile: up to 8 × 6 character cells, each present cell with its own
/// bitmap and attribute.
#[derive(Clone, Debug)]
pub struct Tile {
    /// One byte per row; bit 7 is the leftmost column.
    pub masks: [u8; 6],
    /// Present cells in drawing order (rows top to bottom, columns left to right).
    pub cells: Vec<TileCell>,
}

#[derive(Clone, Copy, Debug)]
pub struct TileCell {
    pub pixels: [u8; 8],
    pub attr: u8,
}

pub struct Assets {
    /// The original program's memory image, for tables not parsed below.
    pub ram: Vec<u8>,
    /// Characters `0x20`–`0x7F`.
    pub font: crate::printer::Font,
    /// User-defined graphics, characters `0x90`–`0xA4`.
    pub udg: [[u8; 8]; 21],
    pub title_udg: [[u8; 8]; 21],
    /// Each room is a 4 × 3 grid of big block indices, row-major.
    pub rooms: Vec<[u8; 12]>,
    /// Each big block is 2 × 2 tile indices, in drawing order: bottom
    /// right, bottom left, top right, top left.
    pub big_blocks: Vec<[u8; 4]>,
    /// Per-tile behaviour byte (colour changes and room objects).
    pub tile_info: [u8; 256],
    pub tiles: Vec<Tile>,
    /// Every blocking sound effect, simulated once: where the speaker
    /// changes, and how long the whole thing takes. Its inputs never change
    /// for the life of the process, and a core piece going into the planet
    /// queues 25 effects in a single frame.
    beeps: Vec<(Vec<(u32, bool)>, u32)>,
    /// The picture the tape showed while the game loaded, if it came from
    /// one. Not drawn by the program itself.
    pub loading_screen: Option<Vec<u8>>,
}

impl Assets {
    /// Parses the tables out of a 64K memory image of the original program.
    ///
    /// # Panics
    ///
    /// If the loaded game data is too short to hold what the original keeps
    /// there, which means the file was not Starquake.
    pub fn from_memory(mem: &[u8]) -> Assets {
        let word = |a: usize| mem[a] as usize | (mem[a + 1] as usize) << 8;

        let rooms = (0..ROOM_COUNT)
            .map(|r| mem[addr::ROOMS + r * 12..][..12].try_into().unwrap())
            .collect();
        let big_blocks = (0..256)
            .map(|b| mem[addr::BIG_BLOCKS + b * 4..][..4].try_into().unwrap())
            .collect();
        let tile_info = mem[addr::TILE_INFO..][..256].try_into().unwrap();

        let tiles = (0..256)
            .map(|t| {
                let p = word(addr::TILE_POINTERS + t * 2);
                let masks: [u8; 6] = std::array::from_fn(|i| mem[(p + i) & 0xFFFF]);
                let count: usize = masks.iter().map(|m| m.count_ones() as usize).sum();
                let cells = (0..count)
                    .map(|k| TileCell {
                        pixels: std::array::from_fn(|i| mem[(p + 6 + k * 8 + i) & 0xFFFF]),
                        // Attributes are stored backwards before the masks.
                        attr: mem[(p + 0xFFFF - k) & 0xFFFF],
                    })
                    .collect();
                Tile { masks, cells }
            })
            .collect();

        Assets {
            ram: mem.to_vec(),
            font: glyphs(mem, addr::FONT + 0x20 * 8),
            udg: glyphs(mem, addr::UDG),
            title_udg: glyphs(mem, addr::TITLE_UDG),
            rooms,
            big_blocks,
            tile_info,
            tiles,
            beeps: (0..EFFECT_COUNT)
                .map(|id| crate::sound::beep(mem, id as u8))
                .collect(),
            loading_screen: None,
        }
    }

    /// A blocking sound effect: its speaker changes and its length. Ids the
    /// game never asks for are silent and instant rather than a panic.
    pub fn beep(&self, id: u8) -> (&[(u32, bool)], u32) {
        match self.beeps.get(id as usize) {
            Some((edges, t)) => (edges, *t),
            None => (&[], 0),
        }
    }

    /// 2 × 2 character graphic `n` (cells top-left, top-right, bottom-left,
    /// bottom-right, 8 bytes each).
    pub fn graphic32(&self, n: u8) -> [u8; 32] {
        self.graphic_at((addr::GRAPHICS32 + n as usize * 32) as u16)
    }

    /// The 2 × 2 character graphic at an address in the original.
    ///
    /// # Panics
    ///
    /// If the loaded game data is too short to hold what the original keeps
    /// there, which means the file was not Starquake.
    pub fn graphic_at(&self, addr: u16) -> [u8; 32] {
        let start = addr as usize;
        self.ram[start..start + 32].try_into().unwrap()
    }
}
