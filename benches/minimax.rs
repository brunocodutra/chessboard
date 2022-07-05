use chessboard::strategy::{Minimax, MinimaxConfig};
use chessboard::{engine::Heuristic, Game, Search};
use criterion::{criterion_group, criterion_main, BatchSize, Criterion};
use proptest::{prelude::*, sample::Selector, strategy::ValueTree, test_runner::TestRunner};

fn game() -> impl Strategy<Value = Game> {
    any::<Selector>().prop_map(|selector| {
        let mut game = Game::default();
        for _ in 0..32 {
            if let Some(m) = selector.try_select(game.position().moves()) {
                game.execute(m.into()).unwrap();
            } else {
                break;
            }
        }
        game
    })
}

fn bench(c: &mut Criterion) {
    let mut runner = TestRunner::default();
    let mut group = c.benchmark_group("minimax");
    for max_depth in [2, 4, 6] {
        let minimax = Minimax::with_config(
            Heuristic::new(),
            MinimaxConfig {
                max_depth,
                ..MinimaxConfig::default()
            },
        );

        group.bench_with_input(format!("depth={}", max_depth), &game(), |b, s| {
            b.iter_batched_ref(
                || s.new_tree(&mut runner).unwrap().current(),
                |game| minimax.search(game),
                BatchSize::SmallInput,
            )
        });
    }
    group.finish();
}

criterion_group!(benches, bench);
criterion_main!(benches);
