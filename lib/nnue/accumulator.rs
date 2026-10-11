use crate::nnue::{Layer, arch};
use crate::simd::*;
use bytemuck::{Zeroable, zeroed};
use derive_more::with_trait::{Debug, Deref, DerefMut};

/// The feature transformer accumulator.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Zeroable, Deref, DerefMut)]
#[debug("Accumulator")]
#[repr(transparent)]
pub struct Accumulator(Aligned<[i16; Accumulator::LEN]>);

impl Default for Accumulator {
    #[inline(always)]
    fn default() -> Self {
        zeroed()
    }
}

impl Layer for Accumulator {
    const LEN: usize = arch::L1;
    type Neuron = i16;
}
