use chessboard::strategy::{Negamax, NegamaxConfig};
use chessboard::{engine::Random, Game, Search};
use criterion::{criterion_group, criterion_main, Criterion};

fn bench(c: &mut Criterion) {
    let mut group = c.benchmark_group("negamax");
    for max_depth in [1, 2, 4] {
        let negamax = Negamax::with_config(Random::new(), NegamaxConfig { max_depth });
        group.bench_function(format!("depth={}", max_depth), |b| {
            b.iter(|| negamax.search(&Game::default()))
        });
    }
    group.finish();
}

criterion_group!(benches, bench);
criterion_main!(benches);
