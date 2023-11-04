use byteorder::{LittleEndian, ReadBytesExt};
use std::{io, mem::transmute};
use zstd::Decoder;

mod accumulator;
mod crelu;
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
pub use crelu::*;
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

/// An [Efficiently Updatable Neural Network][NNUE].
///
/// [NNUE]: https://www.chessprogramming.org/NNUE
#[derive(Debug, Copy, Clone, Eq, PartialEq, Hash)]
pub struct Nnue {
    ft: Transformer<i16, { Self::L0 }, { Self::L1 / 2 }>,
    psqt: Transformer<i32, { Self::L0 }, { Self::PHASES }>,
    output: [CReLU<Output<{ Nnue::L1 }>>; Self::PHASES],
}

impl Nnue {
    const PHASES: usize = 8;
    const L0: usize = 64 * 64 * 11;
    const L1: usize = 1024;

    fn load(bytes: &[u8]) -> io::Result<Box<Self>> {
        let mut nnue: Box<Self> = unsafe { Box::new_zeroed().assume_init() };
        let mut buffer = Decoder::new(bytes)?;

        buffer.read_i16_into::<LittleEndian>(&mut nnue.ft.bias)?;
        buffer.read_i16_into::<LittleEndian>(unsafe {
            transmute::<_, &mut [_; Self::L0 * Self::L1 / 2]>(&mut nnue.ft.weight)
        })?;

        buffer.read_i32_into::<LittleEndian>(unsafe {
            transmute::<_, &mut [_; Self::L0 * Self::PHASES]>(&mut nnue.psqt.weight)
        })?;

        for nn in &mut nnue.output {
            nn.next.bias = buffer.read_i32::<LittleEndian>()?;
            buffer.read_i8_into(&mut nn.next.weight)?;
        }

        debug_assert!(buffer.read_u8().is_err());

        Ok(nnue)
    }
}
