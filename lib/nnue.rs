use crate::util::Int;
use bytemuck::{Zeroable, zeroed};
use std::{ptr, slice};

mod accumulator;
mod evaluator;
mod feature;
mod l12;
mod l23;
mod l3o;
mod layer;
mod phase;
mod synapse;
mod transformer;

pub mod arch;

#[cfg(feature = "permuter")]
pub mod sparsity;

pub use accumulator::*;
pub use evaluator::*;
pub use feature::*;
pub use l3o::*;
pub use l12::*;
pub use l23::*;
pub use layer::*;
pub use phase::*;
pub use synapse::*;
pub use transformer::*;

/// Quantization scale for the feature transformer.
pub const FTQ: i16 = 127;

/// Quantization scale for the hidden layers.
pub const HLQ: i16 = 75;

/// Conversion factor from quantized to floating point.
pub const I2F: f32 = (1 << 7) as f32 / (FTQ as f32 * FTQ as f32 * HLQ as f32);

/// Eval scale.
pub const F2V: f32 = 75.0;

const unsafe fn copy_bytes<T>(dst: &mut T, src: &[u8]) -> usize {
    let len = size_of_val(dst);
    let dst = unsafe { slice::from_raw_parts_mut(ptr::from_mut(dst).cast(), len) };
    dst.copy_from_slice(&src[..len]);
    len
}

static NNUE: Nnue = Nnue::new();

/// An Efficiently Updatable Neural Network.
#[derive(Debug, Zeroable)]
pub struct Nnue {
    transformer: Transformer,
    nn: [L12<L23<L3o>>; Phase::LEN],
}

const impl Nnue {
    fn new() -> Self {
        let bytes = include_bytes!(concat!(env!("OUT_DIR"), "/nnue.bin"));
        let mut nnue: Self = zeroed();
        let mut cursor = 0;

        cursor += unsafe { copy_bytes(&mut nnue.transformer.pp.0, &bytes[cursor..]) };
        cursor += unsafe { copy_bytes(&mut nnue.transformer.ti.0, &bytes[cursor..]) };
        cursor += unsafe { copy_bytes(&mut nnue.transformer.ka.0, &bytes[cursor..]) };
        cursor += unsafe { copy_bytes(&mut nnue.transformer.bias.0, &bytes[cursor..]) };

        for phase in Phase::iter() {
            let nn = &mut nnue.nn[phase];
            cursor += unsafe { copy_bytes(&mut nn.weight.0, &bytes[cursor..]) };
            cursor += unsafe { copy_bytes(&mut nn.bias.0, &bytes[cursor..]) };

            let nn = &mut nn.next;
            cursor += unsafe { copy_bytes(&mut nn.weight.0, &bytes[cursor..]) };
            cursor += unsafe { copy_bytes(&mut nn.bias.0, &bytes[cursor..]) };

            let nn = &mut nn.next;
            cursor += unsafe { copy_bytes(&mut nn.weight.0, &bytes[cursor..]) };
            cursor += unsafe { copy_bytes(&mut nn.bias.0, &bytes[cursor..]) };
        }

        nnue
    }

    #[inline(always)]
    pub fn transformer() -> &'static Transformer {
        &NNUE.transformer
    }

    #[inline(always)]
    pub fn nn(phase: Phase) -> &'static L12<L23<L3o>> {
        &NNUE.nn[phase]
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    #[cfg_attr(miri, ignore)]
    fn feature_transformer_does_not_overflow() {
        let transformer = Nnue::transformer();
        (0..Accumulator::LEN).for_each(|i| {
            let bias = transformer.bias[i] as i32;
            let (mut lower, mut upper) = (bias, bias);

            let mut ka = Vec::from_iter(transformer.ka.iter().map(|a| a[i] as i16));
            let mut ti = Vec::from_iter(transformer.ti.iter().map(|a| a[i] as i16));
            let mut pp = Vec::from_iter(transformer.pp.iter().map(|a| a[i] as i16));

            for (n, ws) in [
                (KAFeature::MAX_ACTIVE, &mut ka),
                (TIFeature::MAX_ACTIVE, &mut ti),
                (PPFeature::MAX_ACTIVE, &mut pp),
            ] {
                let (small, _, _) = ws.select_nth_unstable(n);
                small.iter().for_each(|&v| lower += v as i32);

                let len = ws.len();
                let (_, _, large) = ws.select_nth_unstable(len - 1 - n);
                large.iter().for_each(|&v| upper += v as i32);
            }

            assert!((i16::MIN as i32..=i16::MAX as i32).contains(&lower));
            assert!((i16::MIN as i32..=i16::MAX as i32).contains(&upper));
        });
    }
}
