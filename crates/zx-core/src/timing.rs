//! What a ZX Spectrum frame is made of.
//!
//! Both the reference interpreter and the game work in T-states, the
//! processor's clock ticks, because that is what the original's sound and
//! timing are built from. Keeping the numbers here means the two cannot
//! drift apart.

/// The Z80's clock on a 48K Spectrum, in Hz.
pub const CPU_HZ: u32 = 3_500_000;

/// T-states in one frame. The ULA gives the processor this many between
/// interrupts, and the game's whole sense of time comes from it.
pub const FRAME_T: u32 = 69888;

/// Frames in a second, near enough for pacing. A frame is really 69888 /
/// 3500000 of a second, so the true rate is 50.08 Hz: use [`FRAME_T`] and
/// [`CPU_HZ`] where the difference matters.
pub const FRAMES_PER_SECOND: u32 = 50;

/// How long a frame lasts, to the nanosecond. Not quite 20ms, and the
/// difference is a game running 0.16% slow or fast.
pub const FRAME_NANOS: u64 = FRAME_T as u64 * 1_000_000_000 / CPU_HZ as u64;

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn a_frame_is_just_under_20ms() {
        assert_eq!(FRAME_NANOS, 19_968_000);
        assert!((FRAME_NANOS as f64 / 1e9 * 50.0 - 0.9984).abs() < 1e-9);
    }
}
