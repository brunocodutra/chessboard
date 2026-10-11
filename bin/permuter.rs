use bytemuck::{Zeroable, zeroed, zeroed_vec};
use cinder::nnue::sparsity::{ActivationHistory, ActivationMask};
use cinder::nnue::{Accumulator, Layer};
use cinder::search::{Depth, Engine, Limits};
use cinder::{nnue::arch, util::Num};
use clap::Parser;
use futures::executor::block_on;
use rand::{prelude::*, rng};
use std::{array, iter::repeat_with, time::Instant};

/// The total number of blocks.
const BLOCKS: usize = Accumulator::LEN / 8;

/// The number of neurons per perspective.
const SLOTS: usize = Accumulator::LEN / 2;

/// Number of slots per block.
const BLOCK_SIZE: usize = 4;

/// The number of samples per batch.
const BATCH_SIZE: usize = u64::BITS as usize;

/// A permutation of neuron indices where `self[new] = old`.
type Permutation = [usize; SLOTS];

/// The randomness of one annealing exchange, drawn once per iteration.
#[derive(Debug, Clone, Copy, Zeroable)]
struct Exchange {
    block: usize,
    pair: usize,
    pair_slot: usize,
    block_slot: usize,
    accept: f32,
}

impl Exchange {
    /// Draws an exchange from entropy.
    fn new() -> Self {
        let mut rng = rng();
        let block = rng.random_range(0..BLOCKS);
        let pair = rng.random_range(0..BLOCKS - 1);

        Self {
            block,
            pair: if pair >= block { pair + 1 } else { pair },
            pair_slot: rng.random_range(0..BLOCK_SIZE),
            block_slot: rng.random_range(0..BLOCK_SIZE),
            accept: rng.random(),
        }
    }
}

/// Sample counts per side.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Zeroable)]
struct Counts {
    us: u64,
    them: u64,
}

impl Counts {
    /// The total across both sides.
    fn sum(&self) -> u64 {
        self.us + self.them
    }
}

/// A sample of activation batches.
struct Samples {
    us: Vec<u64>,
    them: Vec<u64>,
    batches: usize,
}

impl Samples {
    /// Transposes batches of [`ActivationMask`]s into per-slot sample columns.
    fn new(history: &[[ActivationMask; BATCH_SIZE]]) -> Self {
        let batches = history.len();
        let mut samples = Samples {
            us: zeroed_vec(SLOTS * batches),
            them: zeroed_vec(SLOTS * batches),
            batches,
        };

        for (batch, chunk) in history.iter().enumerate() {
            for (bit, mask) in chunk.iter().enumerate() {
                let bit = 1u64 << bit;

                for slot in 0..SLOTS {
                    let q = 2 * slot - slot % 8;

                    if mask[q >> 3] >> (q & 7) & 1 == 0 {
                        samples.us[slot * batches + batch] |= bit;
                    }

                    if mask[(q + 8) >> 3] >> ((q + 8) & 7) & 1 == 0 {
                        samples.them[slot * batches + batch] |= bit;
                    }
                }
            }
        }

        samples
    }

    /// The total number of samples.
    fn blocks(&self) -> usize {
        2 * self.batches * BATCH_SIZE * BLOCKS
    }

    /// Counts samples where a block of four is inactive.
    fn count_inactive_block(&self, [i0, i1, i2, i3]: [usize; BLOCK_SIZE]) -> Counts {
        let mut counts: Counts = zeroed();

        for i in 0..self.batches {
            let us = self.us[i0 * self.batches + i]
                & self.us[i1 * self.batches + i]
                & self.us[i2 * self.batches + i]
                & self.us[i3 * self.batches + i];

            let them = self.them[i0 * self.batches + i]
                & self.them[i1 * self.batches + i]
                & self.them[i2 * self.batches + i]
                & self.them[i3 * self.batches + i];

            counts.us += us.count_ones().cast::<u64>();
            counts.them += them.count_ones().cast::<u64>();
        }

        counts
    }

    /// Counts samples where each permuted block is inactive.
    ///
    /// A block is inactive when all four of its neurons are inactive.
    fn count_inactive_blocks(&self, permutation: &Permutation) -> [Counts; BLOCKS] {
        let mut blocks: [Counts; BLOCKS] = zeroed();

        for (block, counts) in blocks.iter_mut().enumerate() {
            let permutation = array::from_fn(|i| permutation[BLOCK_SIZE * block + i]);
            *counts = self.count_inactive_block(permutation);
        }

        blocks
    }

    /// Total number of blocks inactive across all samples under a permutation.
    fn total_inactive_blocks(&self, permutation: &Permutation) -> u64 {
        let inactive_blocks = self.count_inactive_blocks(permutation);
        inactive_blocks.iter().map(Counts::sum).sum()
    }

    /// The fraction of inactive blocks under a permutation.
    fn sparsity(&self, permutation: &Permutation) -> f32 {
        self.total_inactive_blocks(permutation).cast::<f32>() / self.blocks().cast::<f32>()
    }
}

/// Searches for an input layer permutation that maximises block sparsity.
#[derive(Debug, Parser)]
struct Permuter {
    /// The search depth used to record activation masks.
    #[clap(long, default_value_t = Depth::new(16))]
    depth: Depth,

    /// The number of activation masks sampled to score candidate permutations.
    #[clap(long, default_value_t = 200_000)]
    samples: u64,

    /// The number of annealing iterations.
    #[clap(long, default_value_t = 10_000_000)]
    iters: u64,

    /// The initial annealing temperature, as a fraction of the mean inactive block count.
    #[clap(long, default_value_t = 0.02)]
    t_start: f32,

    /// The final annealing temperature, as a fraction of the mean inactive block count.
    #[clap(long, default_value_t = 0.002)]
    t_end: f32,
}

impl Permuter {
    /// Takes the activation record and shuffles it into reference and verification samples.
    fn collect_samples(&self) -> (Samples, Samples) {
        let mut engine = Engine::new();
        block_on(engine.bench(Limits::depth(self.depth)));

        let mut history = ActivationHistory::take();
        history.shuffle(&mut rng());

        let (history, _) = history.as_chunks::<BATCH_SIZE>();
        let samples = self.samples.cast::<usize>() / BATCH_SIZE;
        let (reference, verification) = history.split_at(samples);

        assert!(!reference.is_empty());
        assert!(!verification.is_empty());
        (Samples::new(reference), Samples::new(verification))
    }

    /// Simulated annealing over slot exchanges starting from `permutation`.
    fn anneal(&self, samples: &Samples, mut permutation: Permutation) -> Permutation {
        let mut inactive = samples.count_inactive_blocks(&permutation);
        let mut total = inactive.iter().map(Counts::sum).sum::<u64>().cast::<i64>();
        let mut best = permutation;
        let mut best_total = total;

        let exchanges = repeat_with(Exchange::new).take(self.iters.cast());
        let scale = total.cast::<f32>() / BLOCKS.cast::<f32>() / 2.0;
        assert!(self.t_start > self.t_end);
        assert!(self.t_start > 0.0);
        assert!(self.t_end > 0.0);

        let timer = Instant::now();
        for (iter, exchange) in exchanges.enumerate() {
            let going = permutation[BLOCK_SIZE * exchange.pair + exchange.pair_slot];
            let coming = permutation[BLOCK_SIZE * exchange.block + exchange.block_slot];

            let mut pair_group: [_; BLOCK_SIZE] =
                array::from_fn(|i| permutation[BLOCK_SIZE * exchange.pair + i]);

            let mut block_group: [_; BLOCK_SIZE] =
                array::from_fn(|i| permutation[BLOCK_SIZE * exchange.block + i]);

            pair_group[exchange.pair_slot] = coming;
            block_group[exchange.block_slot] = going;

            let pair_counts = samples.count_inactive_block(pair_group);
            let block_counts = samples.count_inactive_block(block_group);
            let delta = pair_counts.sum().cast::<i64>() + block_counts.sum().cast::<i64>()
                - inactive[exchange.pair].sum().cast::<i64>()
                - inactive[exchange.block].sum().cast::<i64>();

            let decay = iter.cast::<f32>() / (self.iters - 1).max(1).cast::<f32>();
            let temperature = scale * self.t_start * (self.t_end / self.t_start).powf(decay);
            let acceptance = (delta.cast::<f32>() / temperature.max(1e-6)).exp();

            if delta >= 0 || exchange.accept < acceptance {
                permutation[BLOCK_SIZE * exchange.pair + exchange.pair_slot] = coming;
                permutation[BLOCK_SIZE * exchange.block + exchange.block_slot] = going;
                inactive[exchange.pair] = pair_counts;
                inactive[exchange.block] = block_counts;
                total += delta;

                if total > best_total {
                    best = permutation;
                    best_total = total;
                }
            }

            if iter > 0 && iter.is_multiple_of(1_000_000) {
                let elapsed = timer.elapsed().as_secs();
                let remaining = elapsed * (self.iters - iter.cast::<u64>()) / iter.cast::<u64>();
                println!("time remaining: {}m {}s", remaining / 60, remaining % 60);
            }
        }

        best
    }
}

fn main() {
    let permuter = Permuter::parse();

    println!("collecting samples ...");
    let (reference, verification) = permuter.collect_samples();

    println!("annealing ...");
    let current: Permutation = array::from_fn(|index| index);
    let annealed = permuter.anneal(&reference, current);

    println!("current: {:.2}%", verification.sparsity(&current) * 100.0);
    println!("annealed: {:.2}%", verification.sparsity(&annealed) * 100.0);

    if annealed != current && verification.sparsity(&annealed) >= verification.sparsity(&current) {
        let best: Permutation = array::from_fn(|i| arch::PERMUTATION[annealed[i]]);
        println!("permutation: {:?}", best);
    }
}
