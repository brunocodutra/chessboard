use chessboard::strategy::{Minimax, MinimaxConfig};
use chessboard::{Engine, Position, Search, SearchLimits};
use criterion::{criterion_group, criterion_main, BatchSize, Criterion};
use std::time::Duration;

fn bench(c: &mut Criterion) {
    let limits = SearchLimits {
        time: Duration::MAX,
        depth: 5,
    };

    let config = MinimaxConfig {
        search: limits,
        ..MinimaxConfig::default()
    };

    c.benchmark_group("benches").bench_function("minimax", |b| {
        b.iter_batched_ref(
            || Minimax::with_config(Engine::default(), config),
            |s| s.search(&Position::default()).next(),
            BatchSize::SmallInput,
        );
    });
}

criterion_group!(benches, bench);
criterion_main!(benches);
