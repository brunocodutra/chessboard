use crate::chess::{Color, Move};
use crate::util::Assume;
use std::array;
use std::sync::atomic::{AtomicI8, Ordering::Relaxed};

/// [Historical statistics] about a [`Move`].
///
/// [Historical statistics]: https://www.chessprogramming.org/History_Heuristic
#[derive(Debug)]
pub struct History([[[AtomicI8; 2]; 64]; 64]);

impl Default for History {
    #[inline(always)]
    fn default() -> Self {
        History(array::from_fn(|_| {
            array::from_fn(|_| [AtomicI8::new(0), AtomicI8::new(0)])
        }))
    }
}

impl History {
    /// Update statistics about a [`Move`] for a side to move at a given draft.
    #[inline(always)]
    pub fn update(&self, m: Move, side: Color, bonus: i8) {
        let bonus = bonus.max(-i8::MAX);
        let slot = &self.0[m.whence() as usize][m.whither() as usize][side as usize];
        let result = slot.fetch_update(Relaxed, Relaxed, |h| {
            Some((bonus as i16 - bonus.abs() as i16 * h as i16 / 127 + h as i16) as i8)
        });

        result.assume();
    }

    /// Returns the history bonus for a [`Move`].
    #[inline(always)]
    pub fn get(&self, m: Move, side: Color) -> i8 {
        self.0[m.whence() as usize][m.whither() as usize][side as usize].load(Relaxed)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use test_strategy::proptest;

    #[proptest]
    fn update_only_changes_history_of_given_move(
        c: Color,
        b: i8,
        m: Move,
        #[filter((#m.whence(), #m.whither()) != (#n.whence(), #n.whither()))] n: Move,
    ) {
        let h = History::default();
        h.update(m, c, b);
        assert_eq!(h.get(n, c), 0);
    }
}
