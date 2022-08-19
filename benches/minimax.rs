use chessboard::strategy::Minimax;
use chessboard::{Engine, Position, Search, SearchLimits};
use criterion::{criterion_group, criterion_main, BatchSize, Criterion};

fn bench(c: &mut Criterion) {
    use SearchLimits::*;
    c.benchmark_group("benches").bench_function("minimax", |b| {
        b.iter_batched_ref(
            Minimax::<Engine>::default,
            |mm| mm.search(&Position::default(), Depth(5)).next(),
            BatchSize::SmallInput,
        );
    });
}

criterion_group!(benches, bench);
criterion_main!(benches);
