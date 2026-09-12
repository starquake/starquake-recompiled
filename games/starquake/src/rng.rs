//! The game's pseudo-random number generator.
//!
//! Room colours and decoration are drawn from it after reseeding on room
//! entry, so it must reproduce the original sequence exactly.

#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub struct Rng {
    pub a: u16,
    pub b: u16,
    pub c: u16,
    /// Steps until `b` is next advanced.
    pub b_countdown: u8,
    /// Advances of `b` until `c` is next advanced.
    pub c_countdown: u8,
}

impl Rng {
    pub fn step(&mut self) {
        self.a = self
            .a
            .swap_bytes()
            .wrapping_add(self.a)
            .wrapping_add(0x29)
            .wrapping_add(self.b);

        self.b_countdown = self.b_countdown.wrapping_sub(1);
        if self.b_countdown != 0 {
            return;
        }
        self.b_countdown = 5;
        self.b = self.b.wrapping_mul(17).wrapping_add(0xC5).wrapping_add(self.c);

        self.c_countdown = self.c_countdown.wrapping_sub(1);
        if self.c_countdown != 0 {
            return;
        }
        self.c_countdown = 11;
        self.c = self
            .c
            .wrapping_mul(2)
            .wrapping_add(self.a)
            .wrapping_mul(2)
            .wrapping_add(0x4BBB);
    }

    /// Low byte of the main state; most random choices use this.
    pub fn lo(&self) -> u8 {
        self.a as u8
    }

    pub fn hi(&self) -> u8 {
        (self.a >> 8) as u8
    }
}
