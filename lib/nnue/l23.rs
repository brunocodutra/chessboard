use crate::nnue::{L2, L3, Layer, Synapse};
use crate::simd::*;
use bytemuck::{Zeroable, zeroed};
use std::array;

const I: usize = L2::LEN;
const O: usize = (L3::LEN - L2::LEN) * 2;

/// A hidden connection.
#[derive(Debug, Zeroable)]
pub struct L23<S> {
    pub bias: Aligned<[f32; O]>,
    pub weight: Aligned<[[f32; 1]; I * O]>,
    pub next: S,
}

impl<S: for<'a> Synapse<Input<'a> = L3<'a>>> Synapse for L23<S> {
    type Input<'a> = L2<'a>;
    type Output = S::Output;

    #[inline(always)]
    #[cfg_attr(feature = "no_panic", no_panic::no_panic)]
    fn forward(&self, input: Self::Input<'_>) -> Self::Output {
        const { assert!(I.is_multiple_of(W2)) }
        const { assert!(O.is_multiple_of(W2)) }

        const K: usize = unroll(O / W2);
        const { assert!(I.is_multiple_of(K)) }

        let xs: &[[f32; K]; I / K] = input.cast_ref();
        let ws: &[[[V2<f32>; O / W2]; K]; I / K] = self.weight.cast_ref();
        let mut acc = Aligned([[zeroed(); K]; O / W2]);

        for (i, xs) in xs.iter().enumerate() {
            let x: [V2<f32>; K] = array::from_fn(|k| Simd::splat(xs[k]));

            for (j, a) in acc.iter_mut().enumerate() {
                *a = array::from_fn(|k| ws[i][k][j].mul_add(x[k], a[k]));
            }
        }

        let bs: &[[V2<f32>; O / W2 / 2]; 2] = self.bias.cast_ref();
        let acc: &[[[V2<f32>; K]; O / W2 / 2]; 2] = acc.cast_ref();
        let output: [V2<f32>; O / W2 / 2] = array::from_fn(|j| {
            let value = bs[1][j] + acc[1][j].iter().sum::<V2<f32>>();
            let gate = bs[0][j] + acc[0][j].iter().sum::<V2<f32>>();
            let gate = gate.simd_max(zeroed());
            gate * gate * value
        });

        let active = Aligned([output, input.cast()]);
        self.next.forward(active.cast_ref())
    }
}
