use chessboard::{engine::Random, strategy::Negamax, Position, Search, SearchControl};
use criterion::{criterion_group, criterion_main, BatchSize, Criterion};
use proptest::{prelude::*, sample::Selector, strategy::ValueTree, test_runner::TestRunner};

fn position() -> impl Strategy<Value = Position> {
    any::<Selector>().prop_map(|selector| {
        let mut pos = Position::default();
        for _ in 0..8 {
            if let Some(m) = selector.try_select(pos.moves()) {
                pos.play(m).unwrap();
            } else {
                break;
            }
        }
        pos
    })
}

fn bench(c: &mut Criterion) {
    let mut runner = TestRunner::default();
    let mut group = c.benchmark_group("negamax");
    let negamax = Negamax::new(Random::new());
    for d in [1, 2, 4] {
        let ctrl = SearchControl { max_depth: Some(d) };
        group.bench_with_input(format!("depth={}", d), &position(), |b, s| {
            b.iter_batched_ref(
                || s.new_tree(&mut runner).unwrap().current(),
                |pos| negamax.search(pos, ctrl),
                BatchSize::SmallInput,
            )
        });
    }
    group.finish();
}

criterion_group!(benches, bench);
criterion_main!(benches);
