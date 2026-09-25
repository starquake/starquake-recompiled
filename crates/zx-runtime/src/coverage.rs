//! Which of a program's instructions have run, and which way each
//! conditional branch went, across a whole run of checks (starquake-recompiled#121).
//!
//! One byte an address, shared by every machine in the process, so the
//! thousands of machines a check clones all add to the same map. Off until
//! [`start`] is called; while off, a step costs one relaxed load.

use std::sync::atomic::{AtomicBool, AtomicU8, Ordering};

/// The instruction starting here has run.
pub const RAN: u8 = 1;
/// A conditional branch here went to its target.
pub const TAKEN: u8 = 2;
/// A conditional branch here fell through.
pub const NOT_TAKEN: u8 = 4;

static ON: AtomicBool = AtomicBool::new(false);
static MAP: [AtomicU8; 0x10000] = [const { AtomicU8::new(0) }; 0x10000];

/// Clears the map and starts recording.
pub fn start() {
    for cell in &MAP {
        cell.store(0, Ordering::Relaxed);
    }
    ON.store(true, Ordering::Relaxed);
}

/// Whether recording is on.
#[inline]
pub fn on() -> bool {
    ON.load(Ordering::Relaxed)
}

/// Marks `bits` at `pc`.
#[inline]
pub fn mark(pc: u16, bits: u8) {
    MAP[usize::from(pc)].fetch_or(bits, Ordering::Relaxed);
}

/// The map as it stands: a byte an address.
pub fn map() -> Vec<u8> {
    MAP.iter().map(|c| c.load(Ordering::Relaxed)).collect()
}

/// Instructions that have run, and conditional branches taken each way,
/// from `from` up: what a floor compares.
pub fn totals(from: u16) -> (usize, usize) {
    let m = map();
    let above = &m[usize::from(from)..];
    let ran = above.iter().filter(|&&b| b & RAN != 0).count();
    let directions = above
        .iter()
        .map(|&b| usize::from(b & TAKEN != 0) + usize::from(b & NOT_TAKEN != 0))
        .sum();
    (ran, directions)
}
