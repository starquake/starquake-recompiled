//! `.tap` tape loader: the blocks a Spectrum would load from cassette.
//!
//! A tape is a sequence of blocks, each a two-byte length followed by that
//! many bytes: a flag (0x00 for a header, 0xFF for data), the payload, and a
//! checksum. A header says what the block after it is and, for code, where
//! it loads.

/// Bytes of a Spectrum screen: bitmap then attributes.
pub const SCREEN_LEN: usize = 6912;

const HEADER: u8 = 0x00;
const DATA: u8 = 0xFF;
const CODE: u8 = 3;
const SCREEN_ADDR: u16 = 0x4000;

/// A loaded tape.
pub struct Tape {
    /// Contents of 0x4000..=0xFFFF, as the code blocks left it.
    pub ram: Vec<u8>,
    /// The picture shown while the rest of the tape loaded, if it has one.
    pub loading_screen: Option<Vec<u8>>,
}

impl Tape {
    /// Full 64K address space with the RAM in place and zeros for the ROM.
    pub fn memory(&self) -> Vec<u8> {
        let mut mem = vec![0u8; 0x4000];
        mem.extend_from_slice(&self.ram);
        mem
    }
}

/// Loads every code block on a `.tap`, in the order the Spectrum would.
pub fn load_tap(bytes: &[u8]) -> Result<Tape, String> {
    let mut ram = vec![0u8; 0xC000];
    let mut loading_screen = None;
    // Where the block after the current header will load.
    let mut pending: Option<(u16, usize)> = None;
    let mut loaded = 0usize;
    let mut i = 0usize;

    while i + 2 <= bytes.len() {
        let len = bytes[i] as usize | (bytes[i + 1] as usize) << 8;
        i += 2;
        if len < 2 || i + len > bytes.len() {
            return Err(format!("truncated tape block at {i:#x}"));
        }
        let block = &bytes[i..i + len];
        i += len;

        match block[0] {
            HEADER if len >= 19 => {
                pending = (block[1] == CODE).then(|| {
                    let length = block[12] as usize | (block[13] as usize) << 8;
                    let start = block[14] as u16 | (block[15] as u16) << 8;
                    (start, length)
                });
            }
            DATA => {
                let Some((start, length)) = pending.take() else { continue };
                let data = &block[1..len - 1];
                let n = length.min(data.len());
                let at = start as usize;
                if at < 0x4000 || at + n > 0x10000 {
                    return Err(format!("tape block loads outside RAM at {at:#06x}"));
                }
                // The loading picture goes to the screen first, and the game
                // lands on top of it later.
                if at == SCREEN_ADDR as usize && n == SCREEN_LEN && loading_screen.is_none() {
                    loading_screen = Some(data[..n].to_vec());
                }
                ram[at - 0x4000..at - 0x4000 + n].copy_from_slice(&data[..n]);
                loaded += n;
            }
            _ => pending = None,
        }
    }

    if loaded == 0 {
        return Err("no code blocks on the tape".into());
    }
    Ok(Tape { ram, loading_screen })
}
