use criterion::{criterion_group, criterion_main, BatchSize, Criterion};
use lib::search::{Depth, Limits, Options, Searcher};
use lib::{chess::Fen, eval::Evaluator, util::Saturate};
use shakmaty as sm;
use std::thread::available_parallelism;
use std::time::{Duration, Instant};

fn ttm(c: &mut Criterion, name: &str, edps: &[(&str, &str)]) {
    let mut positions = edps.iter().cycle().map(|(p, m)| {
        let fen: Fen = p.parse().unwrap();
        let uci: sm::uci::Uci = m.parse().unwrap();
        (fen.try_into().unwrap(), uci.into())
    });

    let options = match available_parallelism() {
        Err(_) => Options::default(),
        Ok(threads) => Options {
            threads,
            ..Options::default()
        },
    };

    c.benchmark_group("benches")
        .bench_function(format!("ttm/{name}"), |b| {
            b.iter_batched_ref(
                || {
                    (
                        Searcher::with_options(Evaluator::default(), options),
                        positions.next().unwrap(),
                    )
                },
                |(s, (pos, m))| {
                    let timer = Instant::now();
                    for d in 1..=Depth::MAX.get() {
                        let report = s.search(pos, Limits::Depth(Depth::new(d)));
                        if report.pv().first() == Some(m)
                            || timer.elapsed() >= Duration::from_millis(80)
                        {
                            break;
                        }
                    }
                },
                BatchSize::SmallInput,
            );
        });
}

fn bkt(c: &mut Criterion) {
    #[rustfmt::skip]
    // https://www.chessprogramming.org/Bratko-Kopec_Test
    ttm(c, "bkt", &[
        ("1k1r4/pp1b1R2/3q2pp/4p3/2B5/4Q3/PPP2B2/2K5 b - -", "d6d1"),
        ("3r1k2/4npp1/1ppr3p/p6P/P2PPPP1/1NR5/5K2/2R5 w - -", "d4d5"),
        ("2q1rr1k/3bbnnp/p2p1pp1/2pPp3/PpP1P1P1/1P2BNNP/2BQ1PRK/7R b - -", "f6f5"),
        ("rnbqkb1r/p3pppp/1p6/2ppP3/3N4/2P5/PPP1QPPP/R1B1KB1R w KQkq -", "e5e6"),
        ("r1b2rk1/2q1b1pp/p2ppn2/1p6/3QP3/1BN1B3/PPP3PP/R4RK1 w - -", "c3d5"),
        ("2r3k1/pppR1pp1/4p3/4P1P1/5P2/1P4K1/P1P5/8 w - -", "g5g6"),
        ("1nk1r1r1/pp2n1pp/4p3/q2pPp1N/b1pP1P2/B1P2R2/2P1B1PP/R2Q2K1 w - -", "h5f6"),
        ("4b3/p3kp2/6p1/3pP2p/2pP1P2/4K1P1/P3N2P/8 w - -", "f4f5"),
        ("2kr1bnr/pbpq4/2n1pp2/3p3p/3P1P1B/2N2N1Q/PPP3PP/2KR1B1R w - -", "f4f5"),
        ("3rr1k1/pp3pp1/1qn2np1/8/3p4/PP1R1P2/2P1NQPP/R1B3K1 b - -", "c6e5"),
        ("2r1nrk1/p2q1ppp/bp1p4/n1pPp3/P1P1P3/2PBB1N1/4QPPP/R4RK1 w - -", "f2f4"),
        ("r3r1k1/ppqb1ppp/8/4p1NQ/8/2P5/PP3PPP/R3R1K1 b - -", "d7f5"),
        ("r2q1rk1/4bppp/p2p4/2pP4/3pP3/3Q4/PP1B1PPP/R3R1K1 w - -", "b2b4"),
        ("rnb2r1k/pp2p2p/2pp2p1/q2P1p2/8/1Pb2NP1/PB2PPBP/R2Q1RK1 w - -", "d1d2"),
        ("2r3k1/1p2q1pp/2b1pr2/p1pp4/6Q1/1P1PP1R1/P1PN2PP/5RK1 w - -", "g4g7"),
        ("r1bqkb1r/4npp1/p1p4p/1p1pP1B1/8/1B6/PPPN1PPP/R2Q1RK1 w kq -", "d2e4"),
        ("r2q1rk1/1ppnbppp/p2p1nb1/3Pp3/2P1P1P1/2N2N1P/PPB1QP2/R1B2RK1 b - -", "h7h5"),
        ("r1bq1rk1/pp2ppbp/2np2p1/2n5/P3PP2/N1P2N2/1PB3PP/R1B1QRK1 b - -", "c5b3"),
        ("3rr3/2pq2pk/p2p1pnp/8/2QBPP2/1P6/P5PP/4RRK1 b - -", "e8e4"),
        ("r4k2/pb2bp1r/1p1qp2p/3pNp2/3P1P2/2N3P1/PPP1Q2P/2KRR3 w - -", "g3g4"),
        ("3rn2k/ppb2rpp/2ppqp2/5N2/2P1P3/1P5Q/PB3PPP/3RR1K1 w - -", "f5h6"),
        ("2r2rk1/1bqnbpp1/1p1ppn1p/pP6/N1P1P3/P2B1N1P/1B2QPP1/R2R2K1 b - -", "b7e4"),
        ("r1bqk2r/pp2bppp/2p5/3pP3/P2Q1P2/2N1B3/1PP3PP/R4RK1 b kq -", "f7f6"),
        ("r2qnrnk/p2b2b1/1p1p2pp/2pPpp2/1PP1P3/PRNBB3/3QNPPP/5RK1 w - -", "f2f4"),
    ]);
}

fn zugzwang(c: &mut Criterion) {
    #[rustfmt::skip]
    // https://www.chessprogramming.org/Null_Move_Test-Positions
    ttm(c, "zugzwang", &[
        ("8/8/p1p5/1p5p/1P5p/8/PPP2K1p/4R1rk w - -", "e1f1"),
        ("1q1k4/2Rr4/8/2Q3K1/8/8/8/8 w - -", "g5h6"),
        ("7k/5K2/5P1p/3p4/6P1/3p4/8/8 w - -", "f7e7"),
        ("8/6B1/p5p1/Pp4kp/1P5r/5P1Q/4q1PK/8 w - -", "h3h4"),
        ("8/8/1p1r1k2/p1pPN1p1/P3KnP1/1P6/8/3R4 b - -", "f4d5"),
    ]);
}

criterion_group!(benches, bkt, zugzwang);
criterion_main!(benches);
