use crate::chess::Move;

/// A set of [killer moves].
///
/// [killer moves]: https://www.chessprogramming.org/Killer_Move
#[derive(Debug, Default, Copy, Clone, Eq, PartialEq, Hash)]
#[cfg_attr(test, derive(test_strategy::Arbitrary))]
pub struct Killers(Option<Move>, Option<Move>);

impl Killers {
    /// Adds a killer move to the set.
    #[inline(always)]
    pub fn insert(&mut self, m: Move) {
        if self.0 != Some(m) {
            self.1 = self.0;
            self.0 = Some(m);
        }
    }

    /// Whether a move is in the set.
    #[inline(always)]
    pub fn contains(&self, m: Move) -> bool {
        self.0 == Some(m) || self.1 == Some(m)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use proptest::sample::size_range;
    use std::collections::HashSet;
    use test_strategy::proptest;

    #[proptest]
    fn contains_returns_true_only_if_inserted(m: Move) {
        let mut k = Killers::default();
        assert!(!k.contains(m));
        k.insert(m);
        assert!(k.contains(m));
    }

    #[proptest]
    fn insert_avoids_duplicated_moves(m: Move) {
        let mut k = Killers::default();

        k.insert(m);
        k.insert(m);

        assert_eq!(k, Killers(Some(m), None));
    }

    #[proptest]
    fn insert_keeps_most_recent(#[any(size_range(2..10).lift())] ms: HashSet<Move>, m: Move) {
        let mut k = Killers::default();

        for m in ms {
            k.insert(m);
        }

        k.insert(m);
        assert_eq!(k.0, Some(m));
    }
}
