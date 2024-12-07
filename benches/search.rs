#![feature(custom_test_frameworks)]
#![test_runner(criterion::runner)]

use criterion::{Criterion, SamplingMode, Throughput};
use criterion_macro::criterion;
use lib::nnue::Evaluator;
use lib::search::{Depth, Engine, Limits, Options};
use lib::util::{Integer, Trigger};
use std::thread::available_parallelism;
use std::time::{Duration, Instant};

fn bench(reps: u64, options: &Options, limits: &Limits) -> Duration {
    let mut time = Duration::ZERO;

    for _ in 0..reps {
        let mut e = Engine::with_options(options);
        let stopper = Trigger::armed();
        let pos = Evaluator::default();
        let timer = Instant::now();
        e.search::<1>(&pos, limits, &stopper);
        time += timer.elapsed();
    }

    time
}

#[criterion]
fn crit(c: &mut Criterion) {
    let thread_limit = match available_parallelism() {
        Ok(cores) => cores.get().div_ceil(2),
        Err(_) => 1,
    };

    let options = Vec::from_iter((0..=thread_limit.ilog2()).map(|threads| Options {
        threads: 2usize.pow(threads).saturate(),
        ..Options::default()
    }));

    for o in &options {
        let depth = Depth::new(16);
        c.benchmark_group("ttd")
            .sampling_mode(SamplingMode::Flat)
            .bench_function(o.threads.to_string(), |b| {
                b.iter_custom(|i| bench(i, o, &depth.into()))
            });
    }

    for o in &options {
        let nodes = 200_000;
        c.benchmark_group("nps")
            .sampling_mode(SamplingMode::Flat)
            .throughput(Throughput::Elements(nodes))
            .bench_function(o.threads.to_string(), |b| {
                b.iter_custom(|i| bench(i, o, &nodes.into()))
            });
    }
}
