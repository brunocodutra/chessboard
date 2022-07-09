use chessboard::{strategy::Minimax, Engine, Game, Search};
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
    let minimax = Minimax::new(Engine::default());
    c.bench_function("minimax", |b| {
        b.iter_batched_ref(
            || game().new_tree(&mut runner).unwrap().current(),
            |game| minimax.search(game),
            BatchSize::SmallInput,
        )
    });
}

criterion_group!(benches, bench);
criterion_main!(benches);
