use chessboard::{engine::Random, strategy::Negamax, Game, Search, SearchControl};
use criterion::{criterion_group, criterion_main, Criterion};

fn bench(c: &mut Criterion) {
    let mut group = c.benchmark_group("negamax");
    let negamax = Negamax::new(Random::new());
    for d in [1, 2, 4] {
        let ctrl = SearchControl { depth: Some(d) };
        group.bench_function(format!("depth={}", d), |b| {
            b.iter(|| negamax.search(&Game::default(), ctrl))
        });
    }
    group.finish();
}

criterion_group!(benches, bench);
criterion_main!(benches);
