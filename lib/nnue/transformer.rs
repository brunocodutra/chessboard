use crate::nnue::Transformer;
use test_strategy::Arbitrary;

/// An [affine] feature transformer.
///
/// [affine]: https://en.wikipedia.org/wiki/Affine_transformation
#[derive(Debug, Copy, Clone, Eq, PartialEq, Hash, Arbitrary)]
pub struct FeatureTransformer<const I: usize, const O: usize>(
    pub(super) [[i16; O]; I],
    pub(super) [i16; O],
);

impl<const I: usize, const O: usize> Transformer for FeatureTransformer<I, O> {
    type Accumulator = [i16; O];

    fn refresh(&self, features: &[usize], accumulator: &mut [i16; O]) {
        *accumulator = self.1;
        debug_assert!(features.len() <= 32);
        for f in features.iter().take(32) {
            self.add(*f, accumulator)
        }
    }

    fn add(&self, feature: usize, accumulator: &mut [i16; O]) {
        for (i, a) in accumulator.iter_mut().enumerate() {
            *a += self.0[feature][i]
        }
    }

    fn remove(&self, feature: usize, accumulator: &mut [i16; O]) {
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
    fn transformer_selects_weight_matrix(
        #[strategy([[-9i16..=9, -9..=9], [-9..=9, -9..=9], [-9..=9, -9..=9]])] w: [[i16; 2]; 3],
        #[strategy([..3usize, ..3, ..3])] i: [usize; 3],
    ) {
        assert_eq!(
            FeatureTransformer(w, [0; 2]).forward(&i),
            [
                w[i[0]][0] + w[i[1]][0] + w[i[2]][0],
                w[i[0]][1] + w[i[1]][1] + w[i[2]][1],
            ]
        );
    }

    #[proptest]
    fn transformer_adds_bias_vector(w: [[i16; 2]; 3], b: [i16; 2]) {
        assert_eq!(FeatureTransformer(w, b).forward(&[]), b);
    }

    #[proptest]
    #[should_panic]
    fn transformer_panics_if_too_many_inputs(
        #[strategy([[-9i16..=9, -9..=9], [-9..=9, -9..=9], [-9..=9, -9..=9]])] w: [[i16; 2]; 3],
        #[strategy([-9i16..=9, -9..=9])] b: [i16; 2],
        #[any(size_range(33..=99).lift())] i: Vec<usize>,
    ) {
        FeatureTransformer(w, b).forward(&i);
    }

    #[proptest]
    fn add_updates_accumulator(
        #[strategy([[-9i16..=9, -9..=9], [-9..=9, -9..=9], [-9..=9, -9..=9]])] w: [[i16; 2]; 3],
        b: [i16; 2],
        #[strategy([-9i16..=9, -9..=9])] prev: [i16; 2],
        #[strategy(..3usize)] f: usize,
    ) {
        let mut new = prev;
        FeatureTransformer(w, b).add(f, &mut new);
        assert_eq!(new, [prev[0] + w[f][0], prev[1] + w[f][1]]);
    }

    #[proptest]
    fn remove_updates_accumulator(
        #[strategy([[-9i16..=9, -9..=9], [-9..=9, -9..=9], [-9..=9, -9..=9]])] w: [[i16; 2]; 3],
        b: [i16; 2],
        #[strategy([-9i16..=9, -9..=9])] prev: [i16; 2],
        #[strategy(..3usize)] f: usize,
    ) {
        let mut new = prev;
        FeatureTransformer(w, b).remove(f, &mut new);
        assert_eq!(new, [prev[0] - w[f][0], prev[1] - w[f][1]]);
    }
}
