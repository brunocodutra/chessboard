use chessboard::{engine::Random, strategy::Negamax, Game, Search, SearchControl};
use criterion::{criterion_group, criterion_main, BatchSize, Criterion};
use proptest::{prelude::*, sample::Selector, strategy::ValueTree, test_runner::TestRunner};

fn game() -> impl Strategy<Value = Game> {
    any::<Selector>().prop_map(|selector| {
        let mut game = Game::default();
        for _ in 0..8 {
            if let Some(a) = selector.try_select(game.actions()) {
                game.execute(a).unwrap();
            } else {
                break;
            }
        }
        game
    })
}

fn bench(c: &mut Criterion) {
    let mut runner = TestRunner::default();
    let mut group = c.benchmark_group("negamax");
    let negamax = Negamax::new(Random::new());
    for d in [1, 2, 4] {
        let ctrl = SearchControl { depth: Some(d) };
        group.bench_with_input(format!("depth={}", d), &game(), |b, s| {
            b.iter_batched_ref(
                || s.new_tree(&mut runner).unwrap().current(),
                |game| negamax.search(game, ctrl),
                BatchSize::SmallInput,
            )
        });
    }
    group.finish();
}

criterion_group!(benches, bench);
criterion_main!(benches);
