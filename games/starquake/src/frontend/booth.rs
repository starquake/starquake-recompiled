//! Typing a teleport code with a controller (#80). A booth reads its code
//! from the keyboard, five letters, and a pad has no letters; so once a pad
//! button is pressed in a booth, the window shows five slots over it. Up
//! and down step a slot through A to Z, left and right move between slots,
//! A enters the code and B clears it. From guidance level 1, which lists the
//! codes seen this game, X fills the slots with each of them in turn. The
//! code is then typed on the keys the booth reads, a letter at a time, so
//! the booth itself runs unchanged.

/// The code being put together in the slots.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub struct Entry {
    /// Each slot's letter, or `None` while it is empty.
    pub slots: [Option<u8>; 5],
    /// The slot up and down change.
    pub at: usize,
    /// Which of the seen codes X last filled in, if any.
    pub seen: Option<usize>,
    /// Entered, and being typed.
    pub entered: bool,
}

impl Entry {
    /// Up (`true`) or down: the slot's letter a step through A to Z, round
    /// the end; an empty slot starts at A going up and Z going down.
    pub fn step(&mut self, up: bool) {
        if self.entered {
            return;
        }
        let slot = &mut self.slots[self.at];
        *slot = Some(match (*slot, up) {
            (None, true) | (Some(b'Z'), true) => b'A',
            (None, false) | (Some(b'A'), false) => b'Z',
            (Some(c), true) => c + 1,
            (Some(c), false) => c - 1,
        });
        self.seen = None;
    }

    /// Left (`false`) or right: the next slot, stopping at the ends.
    pub fn move_to(&mut self, right: bool) {
        if self.entered {
            return;
        }
        self.at = if right {
            (self.at + 1).min(4)
        } else {
            self.at.saturating_sub(1)
        };
    }

    /// B: every slot empty, back to the first.
    pub fn clear(&mut self) {
        if !self.entered {
            *self = Entry::default();
        }
    }

    /// X: the next of the codes seen this game, into the slots.
    pub fn next_seen(&mut self, seen: &[[u8; 5]]) {
        if self.entered || seen.is_empty() {
            return;
        }
        let k = self.seen.map_or(0, |k| (k + 1) % seen.len());
        self.slots = seen[k].map(Some);
        self.seen = Some(k);
        self.at = 4;
    }

    /// A: the code to type, once every slot has a letter.
    pub fn enter(&mut self) -> Option<[u8; 5]> {
        if self.entered {
            return None;
        }
        let code = [
            self.slots[0]?,
            self.slots[1]?,
            self.slots[2]?,
            self.slots[3]?,
            self.slots[4]?,
        ];
        self.entered = true;
        Some(code)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn letters_go_round_from_either_end() {
        let mut e = Entry::default();
        e.step(true);
        assert_eq!(e.slots[0], Some(b'A'));
        e.step(false);
        assert_eq!(e.slots[0], Some(b'Z'), "A down is Z");
        e.step(true);
        assert_eq!(e.slots[0], Some(b'A'), "Z up is A");
        let mut e = Entry::default();
        e.step(false);
        assert_eq!(e.slots[0], Some(b'Z'), "empty going down starts at Z");
    }

    #[test]
    fn slots_stop_at_the_ends() {
        let mut e = Entry::default();
        e.move_to(false);
        assert_eq!(e.at, 0);
        for _ in 0..9 {
            e.move_to(true);
        }
        assert_eq!(e.at, 4);
    }

    #[test]
    fn a_code_enters_only_when_full() {
        let mut e = Entry::default();
        e.step(true);
        assert_eq!(e.enter(), None, "four slots empty");
        for _ in 0..4 {
            e.move_to(true);
            e.step(true);
            e.step(true);
        }
        assert_eq!(e.enter(), Some(*b"ABBBB"));
        assert_eq!(e.enter(), None, "once");
        e.step(true);
        assert_eq!(e.slots[4], Some(b'B'), "nothing changes once entered");
    }

    #[test]
    fn x_steps_through_the_seen_codes() {
        let seen = [*b"ABCDE", *b"FGHIJ"];
        let mut e = Entry::default();
        e.next_seen(&seen);
        assert_eq!(e.enter().map(|_| ()), Some(()));
        let mut e = Entry::default();
        e.next_seen(&seen);
        e.next_seen(&seen);
        assert_eq!(e.slots, seen[1].map(Some));
        e.next_seen(&seen);
        assert_eq!(e.slots, seen[0].map(Some), "round again");
        let mut e = Entry::default();
        e.next_seen(&[]);
        assert_eq!(e, Entry::default(), "nothing seen, nothing filled");
    }

    #[test]
    fn b_clears() {
        let mut e = Entry::default();
        e.next_seen(&[*b"ABCDE"]);
        e.clear();
        assert_eq!(e, Entry::default());
    }
}
