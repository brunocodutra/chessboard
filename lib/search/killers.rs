use crate::chess::{Color, Move};
use crate::search::Ply;
use crate::util::{Assume, Binary, Bits, Integer};
use derive_more::Debug;
use std::sync::atomic::{AtomicU32, Ordering::Relaxed};
use std::{array, mem::size_of};

/// A pair of [killer moves].
///
/// [killer moves]: https://www.chessprogramming.org/Killer_Move
#[derive(Debug, Default, Copy, Clone, Eq, PartialEq, Hash)]
#[cfg_attr(test, derive(test_strategy::Arbitrary))]
pub struct Killer(Option<Move>, Option<Move>);

impl Killer {
    /// Adds a killer move to the pair.
    #[inline(always)]
    pub fn insert(&mut self, m: Move) {
        if self.0 != Some(m) {
            self.1 = self.0;
            self.0 = Some(m);
        }
    }

    /// Whether a move is a killer.
    #[inline(always)]
    pub fn contains(&self, m: Move) -> bool {
        self.0 == Some(m) || self.1 == Some(m)
    }
}

impl Binary for Killer {
    type Bits = Bits<u32, { 2 * <Option<Move> as Binary>::Bits::BITS }>;

    #[inline(always)]
    fn encode(&self) -> Self::Bits {
        let mut bits = Bits::default();
        bits.push(self.1.encode());
        bits.push(self.0.encode());
        bits
    }

    #[inline(always)]
    fn decode(mut bits: Self::Bits) -> Self {
        Killer(Binary::decode(bits.pop()), Binary::decode(bits.pop()))
    }
}

/// A set of [killer moves] indexed by [`Ply`] and side to move.
///
/// [killer moves]: https://www.chessprogramming.org/Killer_Move
#[derive(Debug)]
#[debug("Killers({})", size_of::<Self>())]
pub struct Killers([[AtomicU32; 2]; Ply::MAX as usize]);

impl Default for Killers {
    #[inline(always)]
    fn default() -> Self {
        Killers(array::from_fn(|_| [AtomicU32::new(0), AtomicU32::new(0)]))
    }
}

impl Killers {
    /// Adds a killer move to the set at a given ply for a given side to move.
    #[inline(always)]
    pub fn insert(&self, ply: Ply, side: Color, m: Move) {
        let slot = &self.0.get(ply.cast::<usize>()).assume()[side.cast::<usize>()];
        let mut killer = Killer::decode(Bits::new(slot.load(Relaxed)));
        killer.insert(m);
        slot.store(killer.encode().get(), Relaxed);
    }

    /// Returns the known killer moves at a given ply for a given side to move.
    #[inline(always)]
    pub fn get(&self, ply: Ply, side: Color) -> Killer {
        let slot = &self.0.get(ply.cast::<usize>()).assume()[side.cast::<usize>()];
        Killer::decode(Bits::new(slot.load(Relaxed)))
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::util::Integer;
    use proptest::sample::size_range;
    use std::collections::HashSet;
    use std::fmt::Debug;
    use test_strategy::proptest;

    #[proptest]
    fn decoding_encoded_killer(k: Killer) {
        assert_eq!(Killer::decode(k.encode()), k);
    }

    #[proptest]
    fn contains_returns_true_only_if_inserted(m: Move) {
        let mut k = Killer::default();
        assert!(!k.contains(m));
        k.insert(m);
        assert!(k.contains(m));
    }

    #[proptest]
    fn insert_avoids_duplicated_moves(m: Move) {
        let mut k = Killer::default();

        k.insert(m);
        k.insert(m);

        assert_eq!(k, Killer(Some(m), None));
    }

    #[proptest]
    fn insert_keeps_most_recent(#[any(size_range(2..10).lift())] ms: HashSet<Move>, m: Move) {
        let mut k = Killer::default();

        for m in ms {
            k.insert(m);
        }

        k.insert(m);
        assert_eq!(k.0, Some(m));
    }

    #[proptest]
    fn get_turns_killers_at_ply_for_the_side_to_move(
        #[filter((0..Ply::MAX).contains(&#p.get()))] p: Ply,
        c: Color,
        m: Move,
    ) {
        let ks = Killers::default();
        ks.insert(p, c, m);
        let k = ks.get(p, c);
        assert_eq!(k.0, Some(m));
    }
}
