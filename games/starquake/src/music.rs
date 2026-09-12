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
///
/// This is the whole routine at 0x30A9, `push bc` and the closing
/// `pop bc` / `ret` included. The caller charges those three separately,
/// since they are the ones the ULA can hold up.
fn multiply_t(hl: u16) -> u32 {
    931 + 13 * hl.count_ones()
}

/// The player's instructions that the ULA can charge for, each spelled out
/// as the machine cycles it really makes.
///
/// The rest of the player is register work and reads of its own tables, all
/// above 0x8000 where the ULA does not reach, so it costs what the manual
/// says and is added plainly. These are the exceptions, and they are charged
/// through `zx_core::bus` — the same functions the reference interpreter
/// uses — so the two cannot drift apart over where inside an instruction the
/// delay falls. Working that out by hand is what left this model a few
/// hundred T-states adrift over a tune.
mod cycle {
    use zx_core::bus::{Cycle, Kind, charge, charge_io};

    /// The stack. The tape's loader opens with `CLEAR 24103` — 0x5E27 —
    /// which puts the machine stack just below the program, inside the
    /// sixteen kilobytes the ULA shares. Every push and pop the player makes
    /// waits on the picture, and it makes six in every half-cycle.
    ///
    /// Which byte of the stack is which does not matter here: the whole of
    /// it is contended, so one address stands for all of them.
    const STACK: u16 = 0x5DE6;

    /// Cycles naming an address the ULA never wants: the player's own code
    /// and tables.
    fn free(t: &mut u32, len: u32) {
        *t += len;
    }

    fn stack(t: &mut u32, kind: Kind) {
        charge(t, Cycle { at: STACK, len: 3, kind });
    }

    /// `PUSH rr`: the opcode, a cycle spent on the refresh address, then the
    /// two bytes written.
    pub fn push(t: &mut u32) {
        free(t, 4 + 1);
        stack(t, Kind::Write);
        stack(t, Kind::Write);
    }

    /// `POP rr`, and `RET`, which is a pop into the program counter.
    pub fn pop(t: &mut u32) {
        free(t, 4);
        stack(t, Kind::Read);
        stack(t, Kind::Read);
    }

    /// `CALL nn`: the opcode and the address, a cycle to think, then the
    /// return address pushed.
    pub fn call(t: &mut u32) {
        free(t, 4 + 3 + 3 + 1);
        stack(t, Kind::Write);
        stack(t, Kind::Write);
    }

    /// `OUT (n),A`: the opcode and the port number, then the I/O cycle.
    ///
    /// The port is A in the high byte and 0xFE in the low, and A holds the
    /// speaker byte here, so the high byte is never in the ULA's own range:
    /// the processor gets one free T-state and then waits three.
    pub fn out_n_a(t: &mut u32, a: u8) {
        free(t, 4 + 3);
        charge_io(t, u16::from(a) << 8 | 0x00FE);
    }

    /// `IN r,(C)`: two opcode fetches, then the I/O cycle.
    ///
    /// The port is BC, and B is the keyboard half-row, which the player
    /// rotates every half-cycle. Seven of the eight rows have a high byte
    /// outside the ULA's own range, but `0x7F` is inside it, and a port in
    /// that range is held up at the start of the cycle as well as in the
    /// middle. So one half-cycle in eight costs more than its neighbours,
    /// and a tune that never noticed would drift.
    pub fn in_r_c(t: &mut u32, row: u8) {
        free(t, 4 + 4);
        charge_io(t, u16::from(row) << 8 | 0x00FE);
    }
}

/// `call da64` and the little routine it lands in, which flips the direction
/// the note is sliding. Two stack writes going in and two reads coming back.
fn slide_call(t: &mut u32) {
    cycle::call(t);
    // ld a,(da41); ld c,a; ld a,78; sub c; ld (da41),a — all in the player's
    // own page, where the ULA does not reach.
    *t += 41;
    cycle::pop(t);
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
        t += 7 + 6;
        cycle::push(&mut t);
        t += 7;
        let note = ram[p];
        p += 1;
        if note == 0 {
            // jr z; pop hl; ei; ret
            t += 12;
            cycle::pop(&mut t);
            t += 4;
            cycle::pop(&mut t);
            break;
        }
        t += 7;

        // The note's period, and how long it lasts.
        t += 4 + 7 + 7 + 4 + 11 + 10 + 11 + 7 + 6 + 7;
        let i = (note & 0x1F) as usize;
        let period = u16::from_le_bytes([ram[NOTES + i * 2], ram[NOTES + i * 2 + 1]]);
        for _ in 0..3 {
            cycle::push(&mut t);
        }
        t += 7 + 4 + 7 + 4 + 20 + 4;
        let scale = (note | 0x1F) as u16;
        cycle::call(&mut t);
        // The ROM routine keeps BC over its loop, so three of its
        // instructions reach the contended stack: `push bc` going in, then
        // `pop bc` and `ret` coming out. 31 T-states of the 931 are those.
        cycle::push(&mut t);
        t += multiply_t(scale) - 31;
        cycle::pop(&mut t);
        cycle::pop(&mut t);
        let mut length = (scale as u32).wrapping_mul(tempo) as u16;

        // How many half-cycles that is: the period is taken off the length
        // until it runs out, four counted each time.
        cycle::pop(&mut t);
        t += 10;
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
        cycle::pop(&mut t);
        cycle::pop(&mut t);
        t += 4 + 10 + 4;
        // B is the keyboard half-row the player will read, reloaded for
        // every note and rotated once per half-cycle.
        let mut row = 0xFEu8;
        let (mut hl, mut de) = (period, period);
        while half_cycles != 0 {
            t += 13;
            out.push((t, speaker & 0x10 != 0));
            cycle::out_n_a(&mut t, speaker);
            t += 7 + 7 + 13;
            speaker = speaker.wrapping_add(0x10) & 0x30;
            // The keyboard is read one half-row per half-cycle; with no key
            // down the player carries on.
            t += 4 + 4;
            cycle::in_r_c(&mut t, row);
            // rlc b: on to the next half-row, which is the port for the next
            // time round.
            row = row.rotate_left(1);
            t += 8 + 4 + 4 + 7 + 12;

            // The two copies of the period swap, and the delay on this one
            // is what sets the pitch.
            std::mem::swap(&mut hl, &mut de);
            t += 4;
            cycle::push(&mut t);
            cycle::push(&mut t);
            let n = if hl == 0 { 65536 } else { hl as u32 };
            t += n * 26 - 5;
            cycle::pop(&mut t);

            hl = if sliding_down { hl.wrapping_sub(1) } else { hl.wrapping_add(1) };
            t += 6 + 10 + 4 + 4;
            if hl >> 8 != 0 {
                t += 12 + 12 + 7 + 73;
                slide_call(&mut t);
                sliding_down = !sliding_down;
            } else if hl & 0xFF != 2 {
                t += 7 + 4 + 4 + 12 + 7 + 73;
                slide_call(&mut t);
                sliding_down = !sliding_down;
            } else {
                // Wound all the way down: the other copy is nudged out
                // instead, and the slide is flipped twice, so it stays.
                t += 7 + 4 + 4 + 7;
                slide_call(&mut t);
                t += 6 + 12;
                slide_call(&mut t);
                de = de.wrapping_add(1);
            }

            cycle::pop(&mut t);
            t += 6 + 4 + 4;
            half_cycles -= 1;
            t += if half_cycles != 0 { 12 } else { 7 };
        }
        // pop hl; jp
        cycle::pop(&mut t);
        t += 10;
    }
    (out, t)
}
