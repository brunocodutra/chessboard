use crate::search::{Pv, ThreadCount};
use crate::util::{Binary, Bits, Integer};
use derive_more::{Deref, Display, Error, From};
use rayon::{prelude::*, ThreadPool, ThreadPoolBuilder};
use std::sync::atomic::{AtomicU64, Ordering};

/// Indicates the search was interrupted upon reaching the configured [`crate::search::Limits`].
#[derive(Debug, Display, Copy, Clone, Eq, PartialEq, Ord, PartialOrd, Hash, Error)]
#[display("the search was interrupted")]
pub struct Interrupted;

/// Whether the search should be [`Interrupted`] or exited early.
#[derive(Debug, Copy, Clone, Eq, PartialEq, Ord, PartialOrd, Hash, From)]
pub enum ControlFlow {
    Interrupt(Interrupted),
    Break,
}

/// A parallel search driver.
#[derive(Debug)]
pub enum Driver {
    Parallel(ThreadPool),
    Sequential,
}

impl Driver {
    /// Constructs a parallel search driver with the given [`ThreadCount`].
    pub fn new(threads: ThreadCount) -> Self {
        match threads.get() {
            1 => Self::Sequential,
            n => Self::Parallel(ThreadPoolBuilder::new().num_threads(n).build().unwrap()),
        }
    }

    /// Drive the search, possibly across multiple threads in parallel.
    ///
    /// The order in which elements are processed and on which thread is unspecified.
    #[inline(always)]
    pub fn drive<M, F>(&self, mut best: Pv, moves: &[M], f: F) -> Result<Pv, Interrupted>
    where
        M: Sync,
        F: Fn(&Pv, &M) -> Result<Pv, ControlFlow> + Sync,
    {
        match self {
            Self::Sequential => {
                for m in moves.iter().rev() {
                    best = match f(&best, m) {
                        Ok(pv) => pv.max(best),
                        Err(ControlFlow::Break) => break,
                        Err(ControlFlow::Interrupt(e)) => return Err(e),
                    };
                }

                Ok(best)
            }

            Self::Parallel(e) => e.install(|| {
                use Ordering::Relaxed;
                let best = AtomicU64::new(IndexedPv(best, u32::MAX).encode().get());
                let result = moves.par_iter().enumerate().rev().try_for_each(|(idx, m)| {
                    let pv = f(&IndexedPv::decode(Bits::new(best.load(Relaxed))), m)?;
                    best.fetch_max(IndexedPv(pv, idx.saturate()).encode().get(), Relaxed);
                    Ok(())
                });

                if matches!(result, Ok(()) | Err(ControlFlow::Break)) {
                    Ok(*IndexedPv::decode(Bits::new(best.into_inner())))
                } else {
                    Err(Interrupted)
                }
            }),
        }
    }
}

#[derive(Debug, Copy, Clone, Eq, PartialEq, Ord, PartialOrd, Hash, Deref)]
#[cfg_attr(test, derive(test_strategy::Arbitrary))]
struct IndexedPv(#[deref] Pv, u32);

impl Binary for IndexedPv {
    type Bits = Bits<u64, 64>;

    #[inline(always)]
    fn encode(&self) -> Self::Bits {
        let mut bits = Bits::default();
        bits.push(self.score().encode());
        bits.push(Bits::<u32, 32>::new(self.1));
        bits.push(self.best().encode());
        bits
    }

    #[inline(always)]
    fn decode(mut bits: Self::Bits) -> Self {
        let best = Binary::decode(bits.pop());
        let idx = bits.pop::<u32, 32>().get();
        let score = Binary::decode(bits.pop());
        Self(Pv::new(score, best), idx)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::{chess::Move, nnue::Value};
    use std::cmp::max;
    use test_strategy::proptest;

    #[proptest]
    fn decoding_encoded_indexed_pv_is_an_identity(pv: IndexedPv) {
        assert_eq!(IndexedPv::decode(pv.encode()), pv);
    }

    #[proptest]
    fn indexed_pv_with_higher_score_is_larger(a: Pv, b: Pv, i: u32) {
        assert_eq!(a < b, IndexedPv(a, i) < IndexedPv(b, i));
    }

    #[proptest]
    fn indexed_pv_with_same_score_but_higher_index_is_larger(pv: Pv, a: u32, b: u32) {
        assert_eq!(a < b, IndexedPv(pv, a) < IndexedPv(pv, b));
    }

    #[proptest]
    fn driver_finds_max_indexed_pv(c: ThreadCount, pv: Pv, ms: Vec<(Move, Value)>) {
        assert_eq!(
            Driver::new(c).drive(pv, &ms, |_, &(m, v)| Ok(Pv::new(v.saturate(), Some(m)))),
            Ok(*ms
                .into_iter()
                .enumerate()
                .map(|(i, (m, v))| IndexedPv(Pv::new(v.saturate(), Some(m)), i as _))
                .fold(IndexedPv(pv, u32::MAX), max))
        )
    }
}
