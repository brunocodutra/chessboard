use derive_more::{Add, AddAssign, Display, Sub, SubAssign};
use std::sync::atomic::{AtomicU64, Ordering};
use std::time::{Duration, Instant};
use test_strategy::Arbitrary;

/// Collected search metrics.
#[derive(
    Debug,
    Display,
    Default,
    Copy,
    Clone,
    Eq,
    PartialEq,
    Hash,
    Arbitrary,
    Add,
    AddAssign,
    Sub,
    SubAssign,
)]
#[display(
    fmt = "time={}ms nodes={}|{:.0}/s hits={}|{:.2}% cuts[tt={}|{:.2}% pv={}|{:.2}% nm={}|{:.2}%]",
    "self.time().as_millis()",
    "self.nodes()",
    "self.nps()",
    "self.tt_hits()",
    "self.tt_hit_rate() * 100.",
    "self.tt_cuts()",
    "self.tt_cut_rate() * 100.",
    "self.pv_cuts()",
    "self.pv_cut_rate() * 100.",
    "self.nm_cuts()",
    "self.nm_cut_rate() * 100."
)]
pub struct Metrics {
    time: Duration,
    nodes: u64,
    tt_hits: u64,
    tt_cuts: u64,
    pv_cuts: u64,
    nm_cuts: u64,
}

impl Metrics {
    /// Time elapsed.
    pub fn time(&self) -> Duration {
        self.time
    }

    /// Nodes counter.
    pub fn nodes(&self) -> u64 {
        self.nodes
    }

    /// Nodes visited per second.
    pub fn nps(&self) -> f64 {
        self.nodes() as f64 / self.time().as_secs_f64()
    }

    /// Transposition table hits.
    pub fn tt_hits(&self) -> u64 {
        self.tt_hits
    }

    /// Transposition table hit rate.
    pub fn tt_hit_rate(&self) -> f64 {
        self.tt_hits() as f64 / self.nodes() as f64
    }

    /// Transposition table cuts counter.
    pub fn tt_cuts(&self) -> u64 {
        self.tt_cuts
    }

    /// Transposition table cut rate.
    pub fn tt_cut_rate(&self) -> f64 {
        self.tt_cuts() as f64 / self.nodes() as f64
    }

    /// Transposition table move cuts counter.
    pub fn pv_cuts(&self) -> u64 {
        self.pv_cuts
    }

    /// Transposition table move cut rate.
    pub fn pv_cut_rate(&self) -> f64 {
        self.pv_cuts() as f64 / self.nodes() as f64
    }

    /// Null move cuts counter.
    pub fn nm_cuts(&self) -> u64 {
        self.nm_cuts
    }

    /// Null move cut rate.
    pub fn nm_cut_rate(&self) -> f64 {
        self.nm_cuts() as f64 / self.nodes() as f64
    }
}

/// A collector for search metrics.
#[derive(Debug)]
pub struct MetricsCounters {
    time: Instant,
    nodes: AtomicU64,
    tt_hits: AtomicU64,
    tt_cuts: AtomicU64,
    pv_cuts: AtomicU64,
    nm_cuts: AtomicU64,
}

impl Default for MetricsCounters {
    fn default() -> Self {
        MetricsCounters {
            time: Instant::now(),
            nodes: AtomicU64::new(0),
            tt_hits: AtomicU64::new(0),
            tt_cuts: AtomicU64::new(0),
            pv_cuts: AtomicU64::new(0),
            nm_cuts: AtomicU64::new(0),
        }
    }
}

impl MetricsCounters {
    /// How much time has elapsed so far.
    pub fn time(&self) -> Duration {
        self.time.elapsed()
    }

    /// Increment nodes counter.
    pub fn node(&self) -> u64 {
        self.nodes.fetch_add(1, Ordering::Relaxed) + 1
    }

    /// Increment transposition table hits counter.
    pub fn tt_hit(&self) -> u64 {
        self.tt_hits.fetch_add(1, Ordering::Relaxed) + 1
    }

    /// Increment transposition table cuts counter.
    pub fn tt_cut(&self) -> u64 {
        self.tt_cuts.fetch_add(1, Ordering::Relaxed) + 1
    }

    /// Increment transposition table move cuts counter.
    pub fn pv_cut(&self) -> u64 {
        self.pv_cuts.fetch_add(1, Ordering::Relaxed) + 1
    }

    /// Increment null move cuts counter.
    pub fn nm_cut(&self) -> u64 {
        self.nm_cuts.fetch_add(1, Ordering::Relaxed) + 1
    }

    /// Returns the metrics collected.
    pub fn snapshot(&mut self) -> Metrics {
        Metrics {
            time: self.time.elapsed(),
            nodes: *self.nodes.get_mut(),
            tt_hits: *self.tt_hits.get_mut(),
            tt_cuts: *self.tt_cuts.get_mut(),
            pv_cuts: *self.pv_cuts.get_mut(),
            nm_cuts: *self.nm_cuts.get_mut(),
        }
    }
}
