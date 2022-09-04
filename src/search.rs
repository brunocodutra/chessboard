use crate::{chess::Position, Pv, SearchLimits};

/// Trait for types that implement adversarial search algorithms.
pub trait Search {
    /// Clear the transposition table.
    fn clear(&mut self);

    /// Searches for the strongest [variation][`Pv`].
    fn search<const N: usize>(&mut self, pos: &Position, limits: SearchLimits) -> Pv<N>;
}

#[cfg(test)]
mockall::mock! {
    #[derive(Debug)]
    pub Search {
        pub fn clear(&mut self);
        pub fn search(&mut self, pos: &Position, limits: SearchLimits) -> Pv<256>;
    }
}

#[cfg(test)]
impl Search for MockSearch {
    fn clear(&mut self) {
        MockSearch::clear(self)
    }

    fn search<const N: usize>(&mut self, pos: &Position, limits: SearchLimits) -> Pv<N> {
        MockSearch::search(self, pos, limits).truncate()
    }
}
