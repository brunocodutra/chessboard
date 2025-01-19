use crate::chess::{Move, Position, Role};
use crate::util::{AlignTo64, Assume};
use derive_more::Debug;
use std::mem::{size_of, MaybeUninit};
use std::sync::atomic::{AtomicI8, Ordering::Relaxed};

#[cfg(test)]
use proptest::prelude::*;

/// [Historical statistics] about a [`Move`].
///
/// [Historical statistics]: https://www.chessprogramming.org/History_Heuristic
#[derive(Debug)]
#[cfg_attr(test, derive(test_strategy::Arbitrary))]
#[debug("History({})", size_of::<Self>())]
pub struct History(
    #[cfg_attr(test, strategy(any::<[i8; 6 * 64 * 64 * 2]>()
        .prop_map(|q| unsafe { std::mem::transmute_copy(&q) })))]
    AlignTo64<[[[[AtomicI8; 6]; 64]; 64]; 2]>,
);

impl Default for History {
    #[inline(always)]
    fn default() -> Self {
        History(unsafe { MaybeUninit::zeroed().assume_init() })
    }
}

impl History {
    /// Update statistics about a [`Move`] for a side to move at a given draft.
    #[inline(always)]
    pub fn update(&self, pos: &Position, m: Move, bonus: i8) {
        let (wc, wt) = (m.whence() as usize, m.whither() as usize);
        let victim = pos[m.whither()].map_or(Role::King, |p| p.role()) as usize;
        let slot = &self.0[pos.turn() as usize][wc][wt][victim];

        let bonus = bonus.max(-i8::MAX);
        let result = slot.fetch_update(Relaxed, Relaxed, |h| {
            Some((bonus as i16 - bonus.abs() as i16 * h as i16 / 127 + h as i16) as i8)
        });

        result.assume();
    }

    /// Returns the history bonus for a [`Move`].
    #[inline(always)]
    pub fn get(&self, pos: &Position, m: Move) -> i8 {
        let (wc, wt) = (m.whence() as usize, m.whither() as usize);
        let victim = pos[m.whither()].map_or(Role::King, |p| p.role()) as usize;
        self.0[pos.turn() as usize][wc][wt][victim].load(Relaxed)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use proptest::sample::Selector;
    use std::fmt::Debug;
    use test_strategy::proptest;

    #[proptest]
    fn update_only_changes_history_of_given_move(
        #[by_ref] h: History,
        #[filter(#pos.outcome().is_none())] pos: Position,
        #[map(|s: Selector| s.select(#pos.moves().flatten()))] m: Move,
        #[map(|s: Selector| s.select(#pos.moves().flatten()))]
        #[filter((#m.whence(), #m.whither()) != (#n.whence(), #n.whither()))]
        n: Move,
        b: i8,
    ) {
        let prev = h.get(&pos, n);
        h.update(&pos, m, b);
        assert_eq!(h.get(&pos, n), prev);
    }
}
