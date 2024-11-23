use crate::{chess::Move, search::Score};
use std::cmp::Ordering;
use std::fmt::{self, Display, Formatter, Write};
use std::ops::{Neg, Shr};

#[cfg(test)]
use proptest::{collection::vec, prelude::*};

/// The [principal variation].
///
/// [principal variation]: https://www.chessprogramming.org/Principal_Variation
#[derive(Debug, Clone, Eq, PartialEq, Hash)]
#[cfg_attr(test, derive(test_strategy::Arbitrary))]
pub struct Pv<const N: usize> {
    score: Score,
    #[cfg_attr(test, strategy(vec(any::<Move>(), ..=N).prop_map(|ms| {
        let mut moves = [None; N];
        for (m, n) in moves.iter_mut().zip(ms) {
            *m = Some(n);
        }
        moves
    })))]
    moves: [Option<Move>; N],
}

impl<const N: usize> Pv<N> {
    /// Constructs a [`Pv`].
    #[inline(always)]
    pub fn new<I: IntoIterator<Item = Move>>(score: Score, ms: I) -> Self {
        let mut moves = [None; N];
        for (m, n) in moves.iter_mut().zip(ms) {
            *m = Some(n);
        }

        Pv { score, moves }
    }

    /// The score from the point of view of the side to move.
    #[inline(always)]
    pub fn score(&self) -> Score {
        self.score
    }

    /// The sequence of [`Move`]s in this principal variation.
    #[inline(always)]
    pub fn moves(&self) -> impl Iterator<Item = Move> + '_ {
        self.moves.iter().map_while(|m| *m)
    }

    /// Converts to a principal variation of a different length.
    #[inline(always)]
    pub fn convert<const M: usize>(self) -> Pv<M> {
        Pv::new(self.score(), self.moves())
    }
}

impl<const N: usize> Ord for Pv<N> {
    #[inline(always)]
    fn cmp(&self, other: &Self) -> Ordering {
        self.score.cmp(&other.score)
    }
}

impl<const N: usize> PartialOrd for Pv<N> {
    #[inline(always)]
    fn partial_cmp(&self, other: &Self) -> Option<Ordering> {
        Some(self.cmp(other))
    }
}

impl<T, const N: usize> PartialEq<T> for Pv<N>
where
    Score: PartialEq<T>,
{
    #[inline(always)]
    fn eq(&self, other: &T) -> bool {
        self.score.eq(other)
    }
}

impl<T, const N: usize> PartialOrd<T> for Pv<N>
where
    Score: PartialOrd<T>,
{
    #[inline(always)]
    fn partial_cmp(&self, other: &T) -> Option<Ordering> {
        self.score.partial_cmp(other)
    }
}

impl<const N: usize> Neg for Pv<N> {
    type Output = Self;

    #[inline(always)]
    fn neg(mut self) -> Self::Output {
        self.score = -self.score;
        self
    }
}

impl<const N: usize> Shr<Pv<N>> for Move {
    type Output = Pv<N>;

    #[inline(always)]
    fn shr(self, mut pv: Pv<N>) -> Self::Output {
        if N > 0 {
            pv.moves.copy_within(..N - 1, 1);
            pv.moves[0] = Some(self);
        }

        pv
    }
}

impl<const N: usize> Display for Pv<N> {
    fn fmt(&self, f: &mut Formatter<'_>) -> fmt::Result {
        let mut moves = self.moves();
        let Some(head) = moves.next() else {
            return Ok(());
        };

        Display::fmt(&head, f)?;

        for m in moves {
            f.write_char(' ')?;
            Display::fmt(&m, f)?;
        }

        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use test_strategy::proptest;

    #[proptest]
    fn score_returns_score(pv: Pv<3>) {
        assert_eq!(pv.score(), pv.score);
    }

    #[proptest]
    fn pv_with_larger_score_is_larger(p: Pv<3>, #[filter(#p.score() != #q.score())] q: Pv<3>) {
        assert_eq!(p < q, p.score() < q.score());
    }

    #[proptest]
    fn negation_changes_score(pv: Pv<3>) {
        assert_eq!(pv.clone().neg().score(), -pv.score());
    }

    #[proptest]
    fn negation_preserves_moves(pv: Pv<3>) {
        assert_eq!(
            pv.moves().collect::<Vec<_>>(),
            pv.neg().moves().collect::<Vec<_>>()
        );
    }

    #[proptest]
    fn shift_preserves_score(pv: Pv<3>, m: Move) {
        assert_eq!(m.shr(pv.clone()).score(), pv.score());
    }

    #[proptest]
    fn shift_changes_moves(pv: Pv<3>, m: Move) {
        assert_eq!(m.shr(pv).moves().next(), Some(m));
    }

    #[proptest]
    fn convert_preserves_score(pv: Pv<3>) {
        assert_eq!(pv.score(), pv.convert::<0>().score());
    }

    #[proptest]
    fn convert_truncates_moves(pv: Pv<3>) {
        assert_eq!(
            pv.moves().take(2).collect::<Vec<_>>(),
            pv.convert::<2>().moves().collect::<Vec<_>>()
        );
    }
}
