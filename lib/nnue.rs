use byteorder::{LittleEndian, ReadBytesExt};
use ruzstd::StreamingDecoder;
use std::io::{self, Read};
use std::mem::{transmute, MaybeUninit};

mod accumulator;
mod evaluator;
mod feature;
mod hidden;
mod material;
mod positional;
mod transformer;
mod value;

pub use accumulator::*;
pub use evaluator::*;
pub use feature::*;
pub use hidden::*;
pub use material::*;
pub use positional::*;
pub use transformer::*;
pub use value::*;

/// An [Efficiently Updatable Neural Network][NNUE].
///
/// [NNUE]: https://www.chessprogramming.org/NNUE
#[derive(Debug, Copy, Clone, Eq, PartialEq, Hash)]
struct Nnue {
    ft: Transformer<i16, { Positional::LEN }>,
    psqt: Transformer<i32, { Material::LEN }>,
    hidden: [Hidden<{ Positional::LEN }>; Material::LEN],
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
    fn load<T: Read>(&mut self, mut reader: T) -> io::Result<()> {
        reader.read_i16_into::<LittleEndian>(&mut *self.ft.bias)?;
        reader.read_i16_into::<LittleEndian>(unsafe {
            transmute::<
                &mut [[_; Positional::LEN]; Feature::LEN],
                &mut [_; Feature::LEN * Positional::LEN],
            >(&mut *self.ft.weight)
        })?;

        reader.read_i32_into::<LittleEndian>(unsafe {
            transmute::<
                &mut [[_; Material::LEN]; Feature::LEN],
                &mut [_; Feature::LEN * Material::LEN],
            >(&mut *self.psqt.weight)
        })?;

        for Hidden { bias, weight } in &mut self.hidden {
            *bias = reader.read_i32::<LittleEndian>()?;
            reader.read_i8_into(unsafe {
                transmute::<&mut [[_; Positional::LEN]; 2], &mut [_; Positional::LEN * 2]>(weight)
            })?;
        }

        debug_assert!(reader.read_u8().is_err());

        Ok(())
    }

    fn psqt() -> &'static Transformer<i32, { Material::LEN }> {
        unsafe { &NNUE.psqt }
    }

    fn ft() -> &'static Transformer<i16, { Positional::LEN }> {
        unsafe { &NNUE.ft }
    }

    fn hidden(phase: usize) -> &'static Hidden<{ Positional::LEN }> {
        debug_assert!(phase < Positional::LEN);
        unsafe { NNUE.hidden.get_unchecked(phase) }
    }
}
