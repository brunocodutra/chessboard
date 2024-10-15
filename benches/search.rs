#![feature(custom_test_frameworks)]
#![test_runner(criterion::runner)]

use criterion::{Criterion, SamplingMode, Throughput};
use criterion_macro::criterion;
use lib::nnue::Evaluator;
use lib::search::{Depth, Engine, Limits, Options};
use lib::util::{Integer, Trigger};
use std::time::{Duration, Instant};
use std::{str::FromStr, thread::available_parallelism};

#[ctor::ctor]
static POSITION: Evaluator =
    Evaluator::from_str("6br/1KNp1n1r/2p2p2/P1ppRP2/1kP3pP/3PBB2/PN1P4/8 w - - 0 1").unwrap();

fn bench(reps: u64, options: &Options, limits: &Limits) -> Duration {
    let mut time = Duration::ZERO;

    for _ in 0..reps {
        let mut e = Engine::with_options(options);
        let interrupter = Trigger::armed();
        let timer = Instant::now();
        e.search(&POSITION, limits, &interrupter);
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
        let depth = Depth::new(12);
        c.benchmark_group("ttd")
            .sampling_mode(SamplingMode::Flat)
            .bench_function(o.threads.to_string(), |b| {
                b.iter_custom(|i| bench(i, o, &depth.into()))
            });
    }

    for o in &options {
        let nodes = 300_000;
        c.benchmark_group("nps")
            .sampling_mode(SamplingMode::Flat)
            .throughput(Throughput::Elements(nodes))
            .bench_function(o.threads.to_string(), |b| {
                b.iter_custom(|i| bench(i, o, &nodes.into()))
            });
    }
}
