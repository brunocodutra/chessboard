use crate::{Affine, CReLU, Damp, FeatureTransformer, Passthrough, Psqt};
use std::mem::{size_of, transmute, transmute_copy, MaybeUninit};

/// A trained [`Nnue`].
pub const NNUE: Nnue = Nnue::new();

type L12<I> = Affine<CReLU<I>, { Nnue::L1 }, { Nnue::L2 }>;
type L23<I> = Affine<CReLU<Damp<I, 64>>, { Nnue::L2 }, { Nnue::L3 }>;
type L3o<I> = Affine<CReLU<Damp<I, 64>>, { Nnue::L3 }, 1>;

/// An [Efficiently Updatable Neural Network][NNUE].
///
/// [NNUE]: https://www.chessprogramming.org/NNUE
#[derive(Debug, Copy, Clone, Eq, PartialEq, Hash)]
pub struct Nnue {
    pub(crate) transformer: FeatureTransformer<{ Self::L0 }, { Self::L1 / 2 }>,
    pub(crate) psqt: Psqt<{ Self::L0 }, { Self::PHASES }>,
    pub(crate) nns: [L3o<L23<L12<Passthrough>>>; Self::PHASES],
}

const fn as_array<T, const N: usize>(slice: &[T], offset: usize) -> &[T; N] {
    assert!(offset + N <= slice.len());
    unsafe { transmute(&slice[offset]) }
}

impl Nnue {
    pub(crate) const PHASES: usize = 8;
    pub(crate) const L0: usize = 64 * 64 * 11;
    pub(crate) const L1: usize = 1024;
    pub(crate) const L2: usize = 16;
    pub(crate) const L3: usize = 32;

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
            const B: usize = size_of::<i16>() * Nnue::L1 / 2;
            let bias = transmute_copy(as_array::<_, B>(bytes, cursor));
            cursor += B;

            const W: usize = size_of::<i16>() * Nnue::L0 * Nnue::L1 / 2;
            let weight = transmute_copy(as_array::<_, W>(bytes, cursor));
            cursor += W;

            FeatureTransformer(weight, bias)
        };

        let psqt = unsafe {
            const W: usize = size_of::<i32>() * Nnue::L0 * Nnue::PHASES;
            let weight = transmute_copy(as_array::<_, W>(bytes, cursor));
            cursor += W;

            Psqt(weight)
        };

        let mut phase = 0;
        let mut nns = [MaybeUninit::<L3o<L23<L12<Passthrough>>>>::uninit(); Nnue::PHASES];

        loop {
            if phase >= Nnue::PHASES {
                assert!(cursor == bytes.len());
                break Nnue {
                    transformer,
                    psqt,
                    nns: unsafe { transmute(nns) },
                };
            }

            let l12 = unsafe {
                const B: usize = size_of::<i32>() * Nnue::L2;
                let bias = transmute_copy(as_array::<_, B>(bytes, cursor));
                cursor += B;

                const W: usize = size_of::<i8>() * Nnue::L1 * Nnue::L2;
                let weight = transmute_copy(as_array::<_, W>(bytes, cursor));
                cursor += W;

                Affine(CReLU(Passthrough), weight, bias)
            };

            let l23 = unsafe {
                const B: usize = size_of::<i32>() * Nnue::L3;
                let bias = transmute_copy(as_array::<_, B>(bytes, cursor));
                cursor += B;

                const W: usize = size_of::<i8>() * Nnue::L2 * Nnue::L3;
                let weight = transmute_copy(as_array::<_, W>(bytes, cursor));
                cursor += W;

                Affine(CReLU(Damp(l12)), weight, bias)
            };

            let l3o = unsafe {
                const B: usize = size_of::<i32>();
                let bias = transmute_copy(as_array::<_, B>(bytes, cursor));
                cursor += B;

                const W: usize = size_of::<i8>() * Nnue::L3;
                let weight = transmute_copy(as_array::<_, W>(bytes, cursor));
                cursor += W;

                Affine(CReLU(Damp(l23)), weight, bias)
            };

            nns[phase].write(l3o);
            phase += 1;
        }
    }
}
