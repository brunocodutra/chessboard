use crate::search::{Depth, Pv};
use derive_more::with_trait::{Deref, DerefMut};
use std::time::Duration;

/// Search statistics.
#[derive(Debug, Default, Clone, Copy)]
#[cfg_attr(test, derive(test_strategy::Arbitrary))]
pub struct Stats {
    /// The depth searched.
    pub depth: Depth,
    /// The deepest ply searched.
    pub seldepth: u16,
    /// The duration searched.
    pub time: Duration,
    /// The number of nodes searched.
    pub nodes: u64,
    /// The number of successful tablebase probes.
    pub tbhits: u64,
    /// The fraction of transposition table slots in use.
    pub hashfull: f32,
}

impl Stats {
    #[inline(always)]
    pub fn new(
        depth: Depth,
        seldepth: u16,
        time: Duration,
        nodes: u64,
        tbhits: u64,
        hashfull: f32,
    ) -> Self {
        Self {
            depth,
            seldepth,
            time,
            nodes,
            tbhits,
            hashfull,
        }
    }
}

/// Information about the search result.
#[derive(Debug, Clone, Copy, Deref, DerefMut)]
#[cfg_attr(test, derive(test_strategy::Arbitrary))]
pub struct Info {
    #[deref]
    #[deref_mut]
    stats: Stats,
    pub pv: Pv,
}

impl Info {
    #[inline(always)]
    pub fn new<const N: usize>(stats: Stats, pv: Pv<N>) -> Self {
        Self {
            stats,
            pv: pv.truncate(),
        }
    }
}

impl From<Pv> for Info {
    #[inline(always)]
    fn from(pv: Pv) -> Self {
        Info::new(Stats::default(), pv)
    }
}
