use chessboard::strategy::MinimaxConfig;
use chessboard::{strategy::Minimax, Engine, Position, Search};
use criterion::measurement::{Measurement, WallTime};
use criterion::{criterion_group, criterion_main, Criterion, SamplingMode};
use std::time::Duration;

fn bench(c: &mut Criterion) {
    c.benchmark_group("benches")
        .sample_size(50)
        .sampling_mode(SamplingMode::Flat)
        .warm_up_time(Duration::from_secs(3))
        .measurement_time(Duration::from_secs(50))
        .bench_function("minimax", |b| {
            b.iter_custom(|iters| {
                let mut pos = Position::default();
                let minimax = Minimax::with_config(
                    Engine::default(),
                    MinimaxConfig {
                        max_depth: 5,
                        ..MinimaxConfig::default()
                    },
                );

                let timer = WallTime.start();

                for i in 0..iters {
                    match minimax.search(&pos) {
                        None => return WallTime.end(timer).mul_f64(iters as f64 / i as f64),
                        Some(a) => {
                            pos.play(a).unwrap();
                        }
                    }
                }

                WallTime.end(timer)
            })
        });
}

criterion_group!(benches, bench);
criterion_main!(benches);
