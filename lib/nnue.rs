use byteorder::{LittleEndian, ReadBytesExt};
use std::{io, mem::transmute};
use zstd::Decoder;

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
mod value;
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
pub use value::*;

use vector::*;

lazy_static::lazy_static! {
    /// A trained [`Nnue`].
    pub static ref NNUE: Box<Nnue> =
        Nnue::load(include_bytes!("nnue/nn.zst")).expect("failed to load the NNUE");
}

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

impl Nnue {
    const PHASES: usize = 8;
    const L0: usize = 64 * 64 * 11;
    const L1: usize = 1024;
    const L2: usize = 16;
    const L3: usize = 32;

    fn load(bytes: &[u8]) -> io::Result<Box<Self>> {
        let mut buffer = Decoder::new(bytes)?;
        let mut nnue: Box<Self> = unsafe { Box::new_zeroed().assume_init() };

        assert_eq!(buffer.read_u32::<LittleEndian>()?, 0xffffffff);
        assert_eq!(buffer.read_u32::<LittleEndian>()?, 0x3c103e72);

        buffer.read_i16_into::<LittleEndian>(&mut nnue.ft.bias)?;
        buffer.read_i16_into::<LittleEndian>(unsafe {
            transmute::<_, &mut [_; Self::L0 * Self::L1 / 2]>(&mut nnue.ft.weight)
        })?;

        buffer.read_i32_into::<LittleEndian>(unsafe {
            transmute::<_, &mut [_; Self::L0 * Self::PHASES]>(&mut nnue.psqt.weight)
        })?;

        for nn in &mut nnue.nns {
            let l12 = &mut nn.next;
            buffer.read_i32_into::<LittleEndian>(&mut l12.bias)?;
            buffer.read_i8_into(unsafe {
                transmute::<_, &mut [_; Self::L1 * Self::L2]>(&mut l12.weight)
            })?;

            let l23 = &mut l12.next.next.next;
            buffer.read_i32_into::<LittleEndian>(&mut l23.bias)?;
            buffer.read_i8_into(unsafe {
                transmute::<_, &mut [_; Self::L2 * Self::L3]>(&mut l23.weight)
            })?;

            let l3o = &mut l23.next.next.next;
            l3o.bias = buffer.read_i32::<LittleEndian>()?;
            buffer.read_i8_into(&mut l3o.weight)?;
        }

        debug_assert!(buffer.read_u8().is_err());

        Ok(nnue)
    }
}
