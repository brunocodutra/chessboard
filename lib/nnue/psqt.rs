use crate::nnue::Transformer;

/// A [piece-square table].
///
/// [piece-square table]: https://www.chessprogramming.org/Piece-Square_Tables
#[derive(Debug, Copy, Clone, Eq, PartialEq, Hash)]
#[cfg_attr(test, derive(test_strategy::Arbitrary))]
pub struct Psqt<const I: usize, const O: usize>(pub(super) [[i32; O]; I]);

impl<const I: usize, const O: usize> Transformer for Psqt<I, O> {
    type Accumulator = [i32; O];

    fn refresh(&self, features: &[usize], accumulator: &mut [i32; O]) {
        *accumulator = [0; O];
        debug_assert!(features.len() <= 32);
        for f in features.iter().take(32) {
            self.add(*f, accumulator)
        }
    }

    fn add(&self, feature: usize, accumulator: &mut [i32; O]) {
        for (i, a) in accumulator.iter_mut().enumerate() {
            *a += self.0[feature][i]
        }
    }

    fn remove(&self, feature: usize, accumulator: &mut [i32; O]) {
        for (i, a) in accumulator.iter_mut().enumerate() {
            *a -= self.0[feature][i]
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::nnue::Layer;
    use proptest::sample::size_range;
    use test_strategy::proptest;

    #[proptest]
    fn psqt_selects_weight_matrix(
        #[strategy([[-9i32..=9, -9..=9], [-9..=9, -9..=9], [-9..=9, -9..=9]])] w: [[i32; 2]; 3],
        #[strategy([..3usize, ..3, ..3])] i: [usize; 3],
    ) {
        assert_eq!(
            Psqt(w).forward(&i),
            [
                w[i[0]][0] + w[i[1]][0] + w[i[2]][0],
                w[i[0]][1] + w[i[1]][1] + w[i[2]][1],
            ]
        );
    }

    #[proptest]
    #[should_panic]
    fn psqt_panics_if_too_many_inputs(
        #[strategy([[-9i32..=9, -9..=9], [-9..=9, -9..=9], [-9..=9, -9..=9]])] w: [[i32; 2]; 3],
        #[any(size_range(33..=99).lift())] i: Vec<usize>,
    ) {
        Psqt(w).forward(&i);
    }

    #[proptest]
    fn add_updates_accumulator(
        #[strategy([[-9i32..=9, -9..=9], [-9..=9, -9..=9], [-9..=9, -9..=9]])] w: [[i32; 2]; 3],
        #[strategy([-9i32..=9, -9..=9])] prev: [i32; 2],
        #[strategy(..3usize)] f: usize,
    ) {
        let mut new = prev;
        Psqt(w).add(f, &mut new);
        assert_eq!(new, [prev[0] + w[f][0], prev[1] + w[f][1]]);
    }

    #[proptest]
    fn remove_updates_accumulator(
        #[strategy([[-9i32..=9, -9..=9], [-9..=9, -9..=9], [-9..=9, -9..=9]])] w: [[i32; 2]; 3],
        #[strategy([-9i32..=9, -9..=9])] prev: [i32; 2],
        #[strategy(..3usize)] f: usize,
    ) {
        let mut new = prev;
        Psqt(w).remove(f, &mut new);
        assert_eq!(new, [prev[0] - w[f][0], prev[1] - w[f][1]]);
    }
}
