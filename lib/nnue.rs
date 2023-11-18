use std::mem::{size_of, transmute, transmute_copy, MaybeUninit};

mod accumulator;
mod affine;
mod crelu;
mod damp;
mod evaluator;
mod fallthrough;
mod feature;
mod layer;
mod material;
mod output;
mod positional;
mod transformer;
mod vector;

pub use accumulator::*;
pub use affine::*;
pub use crelu::*;
pub use damp::*;
pub use evaluator::*;
pub use fallthrough::*;
pub use feature::*;
pub use layer::*;
pub use material::*;
pub use output::*;
pub use positional::*;
pub use transformer::*;

use vector::*;

/// A trained [`Nnue`].
pub const NNUE: Nnue = Nnue::new(include_bytes!("nnue/0cd50043.nnue"));

type L12<N> = CReLU<Affine<Damp<N, 64>, { Nnue::L1 }, { Nnue::L2 }>>;
type L23<N> = CReLU<Affine<Damp<N, 64>, { Nnue::L2 }, { Nnue::L3 }>>;
type L3o = CReLU<Output<{ Nnue::L3 }>>;

/// An [Efficiently Updatable Neural Network][NNUE].
///
/// [NNUE]: https://www.chessprogramming.org/NNUE
#[derive(Debug, Copy, Clone, Eq, PartialEq, Hash)]
pub struct Nnue {
    ft: Transformer<i16, { Self::L0 }, { Self::L1 / 2 }>,
    psqt: Transformer<i32, { Self::L0 }, { Self::PHASES }>,
    nns: [L12<L23<L3o>>; Self::PHASES],
}

#[cfg(not(tarpaulin_include))]
const fn as_array<T, const N: usize>(slice: &[T], offset: usize) -> &[T; N] {
    assert!(offset + N <= slice.len());
    unsafe { transmute(&slice[offset]) }
}

impl Nnue {
    const PHASES: usize = 8;
    const L0: usize = 64 * 64 * 11;
    const L1: usize = 1024;
    const L2: usize = 16;
    const L3: usize = 32;

    #[cfg(not(tarpaulin_include))]
    #[cfg(target_endian = "little")]
    const fn new(bytes: &[u8]) -> Self {
        let mut cursor = 0;

        match u32::from_le_bytes(*as_array(bytes, cursor)) {
            0xffffffff => cursor += size_of::<u32>(),
            _ => panic!("version mismatch"),
        }

        match u32::from_le_bytes(*as_array(bytes, cursor)) {
            0x3c103e72 => cursor += size_of::<u32>(),
            _ => panic!("architecture mismatch"),
        }

        let ft = unsafe {
            const B: usize = size_of::<i16>() * Nnue::L1 / 2;
            let bias = transmute_copy(as_array::<_, B>(bytes, cursor));
            cursor += B;

            const W: usize = size_of::<i16>() * Nnue::L0 * Nnue::L1 / 2;
            let weight = transmute_copy(as_array::<_, W>(bytes, cursor));
            cursor += W;

            Transformer::new(bias, weight)
        };

        let psqt = unsafe {
            const W: usize = size_of::<i32>() * Nnue::L0 * Nnue::PHASES;
            let weight = transmute_copy(as_array::<_, W>(bytes, cursor));
            cursor += W;

            Transformer::new([0; Nnue::PHASES], weight)
        };

        let mut phase = 0;
        let mut nns = [MaybeUninit::<L12<L23<L3o>>>::uninit(); Nnue::PHASES];

        loop {
            if phase >= Nnue::PHASES {
                assert!(cursor == bytes.len());
                break Nnue {
                    ft,
                    psqt,
                    nns: unsafe { transmute(nns) },
                };
            }

            let (l12b, l12w) = unsafe {
                const B: usize = size_of::<i32>() * Nnue::L2;
                let bias = transmute_copy(as_array::<_, B>(bytes, cursor));
                cursor += B;

                const W: usize = size_of::<i8>() * Nnue::L1 * Nnue::L2;
                let weight = transmute_copy(as_array::<_, W>(bytes, cursor));
                cursor += W;

                (bias, weight)
            };

            let (l23b, l23w) = unsafe {
                const B: usize = size_of::<i32>() * Nnue::L3;
                let bias = transmute_copy(as_array::<_, B>(bytes, cursor));
                cursor += B;

                const W: usize = size_of::<i8>() * Nnue::L2 * Nnue::L3;
                let weight = transmute_copy(as_array::<_, W>(bytes, cursor));
                cursor += W;

                (bias, weight)
            };

            let (l3ob, l3ow) = unsafe {
                const B: usize = size_of::<i32>();
                let bias = transmute_copy(as_array::<_, B>(bytes, cursor));
                cursor += B;

                const W: usize = size_of::<i8>() * Nnue::L3;
                let weight = transmute_copy(as_array::<_, W>(bytes, cursor));
                cursor += W;

                (bias, weight)
            };

            let l3o = CReLU::new(Output::new(l3ob, l3ow));
            let l23 = CReLU::new(Affine::new(l23b, l23w, Damp::new(l3o)));
            let l12 = CReLU::new(Affine::new(l12b, l12w, Damp::new(l23)));

            nns[phase].write(l12);
            phase += 1;
        }
    }
}
