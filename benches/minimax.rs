use chessboard::strategy::Minimax;
use chessboard::{Engine, Position, Search, SearchLimits};
use criterion::{criterion_group, criterion_main, BatchSize, Criterion};
use std::time::Duration;

fn bench(c: &mut Criterion) {
    let limits = SearchLimits {
        time: Duration::MAX,
        depth: 5,
    };

    c.benchmark_group("benches").bench_function("minimax", |b| {
        b.iter_batched_ref(
            Minimax::<Engine>::default,
            |mm| mm.search(&Position::default(), limits).next(),
            BatchSize::SmallInput,
        );
    });
}

criterion_group!(benches, bench);
criterion_main!(benches);
