use crate::{Affine, CReLU, Chain, Damp, Feature, FeatureTransformer, Layer, Psqt};
use arrayvec::ArrayVec;
use chess::{Color, Position};
use std::iter::repeat;
use std::mem::{size_of, transmute, transmute_copy, MaybeUninit};

/// A trained [`Nnue`].
pub const NNUE: Nnue = Nnue::new();

const PHASES: usize = 8;
const FEATURES: usize = 64 * 64 * 11;
const L1: usize = 1024;
const L2: usize = 16;
const L3: usize = 32;

type NN = Chain<
    Chain<CReLU, Affine<L1, L2>>,
    Chain<
        Chain<Chain<Damp<64>, CReLU>, Affine<L2, L3>>,
        Chain<Chain<Damp<64>, CReLU>, Affine<L3, 1>>,
    >,
>;

/// An [Efficiently Updatable Neural Network][NNUE].
///
/// [NNUE]: https://www.chessprogramming.org/NNUE
#[derive(Debug, Copy, Clone, Eq, PartialEq, Hash)]
pub struct Nnue {
    transformer: FeatureTransformer<FEATURES, { L1 / 2 }>,
    psqt: Psqt<FEATURES, PHASES>,
    nns: [NN; PHASES],
}

const fn as_array<T, const N: usize>(slice: &[T], offset: usize) -> &[T; N] {
    assert!(offset + N <= slice.len());
    unsafe { transmute(&slice[offset]) }
}

impl Nnue {
    #[cfg(target_endian = "little")]
    const fn new() -> Self {
        let mut cursor = 0;
        let bytes = include_bytes!("0cd50043.nnue");

        match u32::from_le_bytes(*as_array(bytes, cursor)) {
            0xffffffff => cursor += size_of::<u32>(),
            _ => panic!("version mismatch"),
        }

        match u32::from_le_bytes(*as_array(bytes, cursor)) {
            0x3c103e72 => cursor += size_of::<u32>(),
            _ => panic!("architecture mismatch"),
        }

        let transformer = unsafe {
            const B: usize = size_of::<i16>() * L1 / 2;
            let bias = transmute_copy(as_array::<_, B>(bytes, cursor));
            cursor += B;

            const W: usize = size_of::<i16>() * FEATURES * L1 / 2;
            let weight = transmute_copy(as_array::<_, W>(bytes, cursor));
            cursor += W;

            FeatureTransformer::<FEATURES, { L1 / 2 }>(weight, bias)
        };

        let psqt = unsafe {
            const W: usize = size_of::<i32>() * FEATURES * PHASES;
            let weight = transmute_copy(as_array::<_, W>(bytes, cursor));
            cursor += W;

            Psqt::<FEATURES, PHASES>(weight)
        };

        let mut phase = 0;
        let mut nns = [MaybeUninit::<NN>::uninit(); PHASES];

        loop {
            if phase >= PHASES {
                assert!(cursor == bytes.len());
                break Nnue {
                    transformer,
                    psqt,
                    nns: unsafe { transmute(nns) },
                };
            }

            let l12 = unsafe {
                const B: usize = size_of::<i32>() * L2;
                let bias = transmute_copy(as_array::<_, B>(bytes, cursor));
                cursor += B;

                const W: usize = size_of::<i8>() * L1 * L2;
                let weight = transmute_copy(as_array::<_, W>(bytes, cursor));
                cursor += W;

                Affine::<L1, L2>(weight, bias)
            };

            let l23 = unsafe {
                const B: usize = size_of::<i32>() * L3;
                let bias = transmute_copy(as_array::<_, B>(bytes, cursor));
                cursor += B;

                const W: usize = size_of::<i8>() * L2 * L3;
                let weight = transmute_copy(as_array::<_, W>(bytes, cursor));
                cursor += W;

                Affine::<L2, L3>(weight, bias)
            };

            let l3o = unsafe {
                const B: usize = size_of::<i32>();
                let bias = transmute_copy(as_array::<_, B>(bytes, cursor));
                cursor += B;

                const W: usize = size_of::<i8>() * L3;
                let weight = transmute_copy(as_array::<_, W>(bytes, cursor));
                cursor += W;

                Affine::<L3, 1>(weight, bias)
            };

            nns[phase].write(Chain(
                Chain(CReLU, l12),
                Chain(
                    Chain(Chain(Damp, CReLU), l23),
                    Chain(Chain(Damp, CReLU), l3o),
                ),
            ));

            phase += 1;
        }
    }

    #[inline]
    pub fn perspective(pos: &Position, side: Color) -> ArrayVec<usize, 32> {
        pos.iter()
            .zip(repeat(pos.king(side)))
            .map(|((p, s), ks)| Feature(ks, p, s))
            .map(|f| f.index(side))
            .collect()
    }

    #[inline]
    pub fn material(&self, phase: usize, us: &[usize], them: &[usize]) -> i32 {
        (self.psqt.forward(us)[phase] - self.psqt.forward(them)[phase]) / 32
    }

    #[inline]
    pub fn positional(&self, phase: usize, us: &[usize], them: &[usize]) -> i32 {
        let us = self.transformer.forward(us);
        let them = self.transformer.forward(them);
        let l1: [i16; L1] = unsafe { transmute_copy(&[us, them]) };
        self.nns[phase].forward(l1)[0] / 16
    }
}
