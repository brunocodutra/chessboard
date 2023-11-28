use lib::chess::Position;
use rayon::prelude::*;
use test_strategy::proptest;

fn perft(pos: &Position, depth: u8) -> usize {
    match depth {
        0 => 1,
        1 => pos.moves().count(),
        d => pos
            .moves()
            .par_bridge()
            .map(|m| {
                let mut next = pos.clone();
                next.play(m);
                perft(&next, d - 1)
            })
            .sum(),
    }
}

#[cfg(not(tarpaulin))]
#[proptest(cases = 1)]
fn perft_expands_expected_number_of_nodes() {
    // https://www.chessprogramming.org/Perft_Results#Initial_Position
    assert_eq!(perft(&Position::default(), 5), 4865609);

    // https://www.chessprogramming.org/Perft_Results#Position_2
    let pos = "r3k2r/p1ppqpb1/bn2pnp1/3PN3/1p2P3/2N2Q1p/PPPBBPPP/R3K2R w KQkq - 0 1".parse()?;
    assert_eq!(perft(&pos, 5), 193690690);

    // https://www.chessprogramming.org/Perft_Results#Position_3
    let pos = "8/2p5/3p4/KP5r/1R3p1k/8/4P1P1/8 w - - 0 1".parse()?;
    assert_eq!(perft(&pos, 5), 674624);

    // https://www.chessprogramming.org/Perft_Results#Position_4
    let pos = "r3k2r/Pppp1ppp/1b3nbN/nP6/BBP1P3/q4N2/Pp1P2PP/R2Q1RK1 w kq - 0 1".parse()?;
    assert_eq!(perft(&pos, 5), 15833292);

    // https://www.chessprogramming.org/Perft_Results#Position_5
    let pos = "rnbq1k1r/pp1Pbppp/2p5/8/2B5/8/PPP1NnPP/RNBQK2R w KQ - 1 8".parse()?;
    assert_eq!(perft(&pos, 5), 89941194);

    // https://www.chessprogramming.org/Perft_Results#Position_6
    let pos = "r4rk1/1pp1qppp/p1np1n2/2b1p1B1/2B1P1b1/P1NP1N2/1PP1QPPP/R4RK1 w - - 0 10".parse()?;
    assert_eq!(perft(&pos, 5), 164075551);
}
