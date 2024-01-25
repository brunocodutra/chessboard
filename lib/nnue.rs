use byteorder::{LittleEndian, ReadBytesExt};
use ruzstd::StreamingDecoder;
use std::io::{self, Read};
use std::mem::{transmute, MaybeUninit};

mod accumulator;
mod evaluator;
mod feature;
mod layer;
mod material;
mod output;
mod positional;
mod transformer;
mod value;

pub use accumulator::*;
pub use evaluator::*;
pub use feature::*;
pub use layer::*;
pub use material::*;
pub use output::*;
pub use positional::*;
pub use transformer::*;
pub use value::*;

/// An [Efficiently Updatable Neural Network][NNUE].
///
/// [NNUE]: https://www.chessprogramming.org/NNUE
#[derive(Debug, Copy, Clone, Eq, PartialEq, Hash)]
struct Nnue {
    ft: Transformer<i16, { Self::L0 }, { Self::L1 / 2 }>,
    psqt: Transformer<i32, { Self::L0 }, { Self::PHASES }>,
    output: [Output<{ Nnue::L1 }>; Self::PHASES],
}

static mut NNUE: Nnue = unsafe { MaybeUninit::zeroed().assume_init() };

#[cold]
#[ctor::ctor]
#[inline(never)]
unsafe fn init() {
    let encoded = include_bytes!("nnue/nn.zst").as_slice();
    let decoder = StreamingDecoder::new(encoded).expect("failed to initialize zstd decoder");
    NNUE.load(decoder).expect("failed to load the NNUE");
}

impl Nnue {
    const PHASES: usize = 8;
    const L0: usize = 64 * 64 * 11;
    const L1: usize = 1024;

    fn load<T: Read>(&mut self, mut reader: T) -> io::Result<()> {
        reader.read_i16_into::<LittleEndian>(&mut *self.ft.bias)?;
        reader.read_i16_into::<LittleEndian>(unsafe {
            transmute::<_, &mut [_; Self::L0 * Self::L1 / 2]>(&mut *self.ft.weight)
        })?;

        reader.read_i32_into::<LittleEndian>(unsafe {
            transmute::<_, &mut [_; Self::L0 * Self::PHASES]>(&mut *self.psqt.weight)
        })?;

        for nn in &mut self.output {
            nn.bias = reader.read_i32::<LittleEndian>()?;
            reader.read_i8_into(&mut *nn.weight)?;
        }

        debug_assert!(reader.read_u8().is_err());

        Ok(())
    }
}
