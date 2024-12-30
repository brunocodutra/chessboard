use crate::search::{Interrupted, Pv, Score, ThreadCount};
use crate::util::{Assume, Integer};
use crate::{chess::Move, nnue::Value};
use derive_more::From;
use rayon::{prelude::*, ThreadPool, ThreadPoolBuilder};
use std::cmp::max_by_key;
use std::sync::atomic::{AtomicI16, Ordering};

/// Whether the search should be [`Interrupted`] or exited early.
#[derive(Debug, Clone, Eq, PartialEq, Ord, PartialOrd, Hash, From)]
pub enum ControlFlow {
    Interrupt(Interrupted),
    Continue,
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
    #[inline(always)]
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
    pub fn drive<F, const N: usize>(
        &self,
        mut head: Move,
        mut tail: Pv<N>,
        moves: &[(Move, Value)],
        f: F,
    ) -> Result<(Move, Pv<N>), Interrupted>
    where
        F: Fn(Score, Move, Value, usize) -> Result<Pv<N>, ControlFlow> + Sync,
    {
        match self {
            Self::Sequential => {
                for (idx, &(m, gain)) in moves.iter().rev().enumerate() {
                    match f(tail.score(), m, gain, idx) {
                        Err(ControlFlow::Break) => break,
                        Err(ControlFlow::Continue) => continue,
                        Err(ControlFlow::Interrupt(e)) => return Err(e),
                        Ok(partial) => {
                            if partial > tail {
                                (head, tail) = (m, partial)
                            }
                        }
                    };
                }

                Ok((head, tail))
            }

            Self::Parallel(e) => e.install(|| {
                use Ordering::Relaxed;
                let score = AtomicI16::new(tail.score().get());
                let (head, tail, _) = moves
                    .par_iter()
                    .rev()
                    .enumerate()
                    .map(|(idx, &(m, gain))| {
                        match f(Score::new(score.load(Relaxed)), m, gain, idx) {
                            Err(ControlFlow::Break) => None,
                            Err(ControlFlow::Continue) => Some(Ok(None)),
                            Err(ControlFlow::Interrupt(e)) => Some(Err(e)),
                            Ok(partial) => {
                                score.fetch_max(partial.score().get(), Relaxed);
                                Some(Ok(Some((m, partial, usize::MAX - idx))))
                            }
                        }
                    })
                    .while_some()
                    .chain([Ok(Some((head, tail, usize::MAX)))])
                    .try_reduce(
                        || None,
                        |a, b| {
                            Ok(max_by_key(a, b, |x| {
                                x.as_ref().map(|(_, t, i)| (t.score(), *i))
                            }))
                        },
                    )?
                    .assume();

                Ok((head, tail))
            }),
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::{chess::Move, nnue::Value};
    use test_strategy::proptest;

    #[proptest]
    fn driver_finds_pv(c: ThreadCount, h: Move, t: Pv<3>, ms: Vec<(Move, Value)>) {
        let (head, tail, _) = ms
            .iter()
            .enumerate()
            .map(|(i, &(m, v))| (m, Pv::new(v.saturate(), []), i))
            .fold((h, t.clone(), usize::MAX), |a, b| {
                max_by_key(a, b, |(_, t, i)| (t.score(), *i))
            });

        assert_eq!(
            Driver::new(c).drive(h, t, &ms, |_, _, v, _| Ok(Pv::new(v.saturate(), []))),
            Ok((head, tail))
        )
    }
}
