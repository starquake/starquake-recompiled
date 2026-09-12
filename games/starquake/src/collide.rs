//! Collision with scenery, which works on the attribute map: a cell whose
//! attribute is below `0x40` (not bright) is solid.

use crate::game::Game;

/// Flags returned by [`Game::collide`].
pub mod blocked {
    pub const RIGHT: u8 = 0x01;
    pub const LEFT: u8 = 0x02;
    pub const BELOW: u8 = 0x04;
    pub const ABOVE: u8 = 0x08;
}

/// Address (Spectrum memory) of the attribute cell under pixel (x, y),
/// y counted from the bottom.
pub fn attr_addr(x: u8, y: u8) -> u16 {
    0x5800 + crate::display::attr_index(x, y) as u16
}

impl Game {
    /// Byte of display memory at a Spectrum address (bitmap, attributes or
    /// the guard row after them).
    ///
    /// Several routines look further ahead than the guard row when BLOB is
    /// near the bottom of a room: `build_platform` probes 64 + 32 bytes past
    /// the cell it starts from, which lands in the restore list. The original
    /// simply read whatever was there, so this does too rather than clamping
    /// to a guess. Beyond the restore list are system variables the rewrite
    /// does not model; nothing reaches that far, and a non-solid byte is the
    /// safe answer if anything ever does.
    pub fn screen_byte(&self, addr: u16) -> u8 {
        if let Some(at) = addr.checked_sub(0x4000)
            && let Some(&b) = self.display.mem.get(at as usize)
        {
            return b;
        }
        let slot = addr.wrapping_sub(crate::room::RESTORE_START) as usize;
        self.restore_mem.get(slot).copied().unwrap_or(0xFF)
    }

    fn solid(&self, addr: u16) -> bool {
        self.screen_byte(addr) < 0x40
    }

    /// Tests the cells beside (`vertical = false`) or above and below
    /// (`vertical = true`) the 2 × 2-cell entity in `slot`. Only done when
    /// the entity is aligned to the grid on that axis; otherwise nothing is
    /// blocked. Returns [`blocked`] flags.
    pub fn collide(&mut self, slot: usize, vertical: bool) -> u8 {
        let (x, y) = (self.entities[slot].x(), self.entities[slot].y());
        let at = attr_addr(x, y);
        self.collision = [0, at as u8, (at >> 8) as u8];
        let wide = x & 7 != 0;
        let tall = y.wrapping_add(1) & 7 != 0;

        if vertical {
            if tall {
                return 0;
            }
            // Two columns above and below, or three when it straddles one.
            let cols = if wide { 3 } else { 2 };
            let row_solid = |start: u16| (0..cols).any(|c| self.solid(start.wrapping_add(c)));
            let (above, below) = (row_solid(at.wrapping_sub(32)), row_solid(at.wrapping_add(64)));
            if above {
                self.collision[0] |= blocked::ABOVE;
            }
            // (The original returns the below flag without storing it.)
            self.collision[0] | if below { blocked::BELOW } else { 0 }
        } else {
            if wide {
                return 0;
            }
            // Two rows beside the entity, or three when it straddles a row.
            let rows = if tall { 3 } else { 2 };
            let column_solid = |start: u16| (0..rows).any(|r| self.solid(start.wrapping_add(32 * r)));
            let (left, right) = (column_solid(at.wrapping_sub(1)), column_solid(at.wrapping_add(2)));
            if left {
                self.collision[0] |= blocked::LEFT;
            }
            // (The original returns the right flag without storing it.)
            self.collision[0] | if right { blocked::RIGHT } else { 0 }
        }
    }
}
