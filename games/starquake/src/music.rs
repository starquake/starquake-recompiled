//! The music player.
//!
//! The original plays its tunes by toggling the speaker inside a delay
//! loop: the note's period is the loop count, and after every half-cycle
//! the count is slid one step up or down, alternately. It keeps two copies
//! of the period and swaps them each time, so the two drift apart and the
//! note buzzes — which is what the tunes sound like.
//!
//! As with the sound effects, this follows the original instruction by
//! instruction and reports where the speaker changes, so the pitch, the
//! buzz and the tempo all come out the same.

/// Note periods: 32 words, indexed by the low five bits of a note.
const NOTES: usize = 0xDA70;

/// What the ROM's multiply costs, which is how long the game takes to work
/// out a note's length. It shifts and adds, so it spends the same again on
/// every set bit of the number being multiplied; the figures come from
/// running the real ROM (see `sq-verify probe`).
fn multiply_t(hl: u16) -> u32 {
    931 + 13 * hl.count_ones()
}

/// Speaker changes of a tune, as (T-state offset, level) pairs, and the
/// tune's whole length in T-states. The tune is generated as though no key
/// is ever pressed; the caller stops it when one is.
pub fn tune(ram: &[u8], addr: usize) -> (Vec<(u32, bool)>, u32) {
    let mut out = Vec::new();
    // di, the pointer patch, inc hl.
    let mut t: u32 = 4 + 16 + 6;
    // The first byte of a tune sets the tempo; the notes follow it.
    let tempo = ram[addr] as u32;
    let mut p = addr + 1;
    // The byte written to the speaker port cycles 0x00, 0x10, 0x20, 0x30,
    // so bit 4 — the speaker — changes every time.
    let mut speaker: u8 = 0;
    // The instruction that slides the pitch is rewritten between `dec hl`
    // and `inc hl` after every half-cycle.
    let mut sliding_down = true;

    loop {
        // ld a,(hl); inc hl; push hl; cp 0
        t += 7 + 6 + 11 + 7;
        let note = ram[p];
        p += 1;
        if note == 0 {
            // jr z; pop hl; ei; ret
            t += 12 + 10 + 4 + 10;
            break;
        }
        t += 7;

        // The note's period, and how long it lasts.
        t += 4 + 7 + 7 + 4 + 11 + 10 + 11 + 7 + 6 + 7;
        let i = (note & 0x1F) as usize;
        let period = u16::from_le_bytes([ram[NOTES + i * 2], ram[NOTES + i * 2 + 1]]);
        t += 33 + 7 + 4 + 7 + 4 + 20 + 4;
        let scale = (note | 0x1F) as u16;
        t += 17 + multiply_t(scale);
        let mut length = (scale as u32).wrapping_mul(tempo) as u16;

        // How many half-cycles that is: the period is taken off the length
        // until it runs out, four counted each time.
        t += 10 + 10;
        let mut half_cycles: u32 = 0;
        loop {
            t += 24 + 4 + 15;
            half_cycles += 4;
            let (next, borrow) = length.overflowing_sub(period);
            length = next;
            if borrow {
                t += 7;
                break;
            }
            t += 12;
        }

        // pop hl; pop de; exx; ld bc,$fefe; exx
        t += 10 + 10 + 4 + 10 + 4;
        let (mut hl, mut de) = (period, period);
        while half_cycles != 0 {
            t += 13;
            out.push((t, speaker & 0x10 != 0));
            t += 11 + 7 + 7 + 13;
            speaker = speaker.wrapping_add(0x10) & 0x30;
            // The keyboard is read one half-row per half-cycle; with no key
            // down the player carries on.
            t += 4 + 4 + 12 + 8 + 4 + 4 + 7 + 12;

            // The two copies of the period swap, and the delay on this one
            // is what sets the pitch.
            std::mem::swap(&mut hl, &mut de);
            t += 4 + 11 + 11;
            let n = if hl == 0 { 65536 } else { hl as u32 };
            t += n * 26 - 5;
            t += 10;

            hl = if sliding_down { hl.wrapping_sub(1) } else { hl.wrapping_add(1) };
            t += 6 + 10 + 4 + 4;
            if hl >> 8 != 0 {
                t += 12 + 12 + 7 + 73 + 68;
                sliding_down = !sliding_down;
            } else if hl & 0xFF != 2 {
                t += 7 + 4 + 4 + 12 + 7 + 73 + 68;
                sliding_down = !sliding_down;
            } else {
                // Wound all the way down: the other copy is nudged out
                // instead, and the slide is flipped twice, so it stays.
                t += 7 + 4 + 4 + 7 + 68 + 6 + 12 + 68;
                de = de.wrapping_add(1);
            }

            t += 10 + 6 + 4 + 4;
            half_cycles -= 1;
            t += if half_cycles != 0 { 12 } else { 7 };
        }
        // pop hl; jp
        t += 10 + 10;
    }
    (out, t)
}
