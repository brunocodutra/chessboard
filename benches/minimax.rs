use chessboard::{strategy::Minimax, Engine, Game, Search};
use criterion::measurement::{Measurement, WallTime};
use criterion::{criterion_group, criterion_main, Criterion, SamplingMode};
use std::time::Duration;

fn bench(c: &mut Criterion) {
    c.benchmark_group("benches")
        .sample_size(10)
        .sampling_mode(SamplingMode::Flat)
        .warm_up_time(Duration::from_secs(3))
        .measurement_time(Duration::from_secs(30))
        .bench_function("minimax", |b| {
            b.iter_custom(|iters| {
                let mut game = Game::default();
                let minimax = Minimax::new(Engine::default());

                let timer = WallTime.start();

                for i in 0..iters {
                    match minimax.search(&game) {
                        None => return WallTime.end(timer).mul_f64(iters as f64 / i as f64),
                        Some(a) => {
                            game.execute(a).unwrap();
                        }
                    }
                }

                WallTime.end(timer)
            })
        });
}

criterion_group!(benches, bench);
criterion_main!(benches);
