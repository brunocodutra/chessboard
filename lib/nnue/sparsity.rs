use crate::nnue::{Accumulator, Layer};
use crate::util::{Assume, Num};
use bytemuck::{Zeroable, zeroed};
use derive_more::with_trait::{Deref, DerefMut};
use std::mem;
use std::ops::BitAnd;
use std::sync::{LazyLock, Mutex};

/// An Activation mask.
///
/// Holds one bit per neuron, set when the neuron is active.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Zeroable, Deref, DerefMut)]
#[repr(transparent)]
pub struct ActivationMask([u8; Accumulator::LEN / u8::BITS as usize]);

impl ActivationMask {
    /// Constructs [`ActivationMask`] from active neurons.
    pub fn new(active: &[u8; Accumulator::LEN]) -> Self {
        let mut mask: Self = zeroed();
        for (i, byte) in mask.iter_mut().enumerate() {
            for j in 0..8 {
                *byte |= u8::from(active[8 * i + j] != 0) << j;
            }
        }

        mask
    }

    /// Relative block sparsity.
    pub fn block_sparsity(&self) -> f32 {
        let mut zeroed = 0u64;
        for byte in self.iter() {
            zeroed += byte.bitand(0x0F).eq(&0).cast::<u64>();
            zeroed += byte.bitand(0xF0).eq(&0).cast::<u64>();
        }

        zeroed.cast::<f32>() / Accumulator::LEN.cast::<f32>() / 4.0
    }
}

/// A global record of [`ActivationMask`]s.
#[derive(Debug, Deref)]
pub struct ActivationHistory(#[deref(forward)] LazyLock<Mutex<Vec<ActivationMask>>>);

static MASKS: ActivationHistory =
    ActivationHistory(LazyLock::new(|| Mutex::new(Vec::with_capacity(1 << 20))));

impl ActivationHistory {
    /// Records a mask for this activation.
    pub fn record(active: &[u8; Accumulator::LEN]) {
        MASKS.lock().assume().push(ActivationMask::new(active));
    }

    /// Takes recorded activation masks.
    pub fn take() -> Vec<ActivationMask> {
        mem::take(&mut *MASKS.lock().assume())
    }
}
