use crate::{Action, Binary, Bits, Cache, Eval, Game, Register, Search};
use bitvec::field::BitField;
use derive_more::{Display, Error, From};
use rayon::prelude::*;
use serde::{Deserialize, Serialize};
use std::sync::atomic::{AtomicI16, Ordering};
use std::{fmt::Debug, str::FromStr};

#[derive(Debug, Copy, Clone, Eq, PartialEq, Ord, PartialOrd, Hash)]
#[cfg_attr(test, derive(test_strategy::Arbitrary))]
enum SearchResultKind {
    Lower,
    Upper,
    Exact,
}

#[derive(Debug, Copy, Clone, Eq, PartialEq, Hash)]
#[cfg_attr(test, derive(test_strategy::Arbitrary))]
struct SearchResult {
    kind: SearchResultKind,
    score: i16,
    #[cfg_attr(test, strategy(0i8..))]
    height: i8,
    action: Action,
    signature: Bits<u32, 24>,
}

/// The reason why decoding [`SearchResult`] from binary failed.
#[derive(Debug, Display, Clone, Eq, PartialEq, Hash, Error)]
#[cfg_attr(test, derive(test_strategy::Arbitrary))]
#[display(fmt = "`{}` is not a valid search result", _0)]
struct DecodeSearchResultError(#[error(not(source))] Bits<u64, 64>);

impl Binary for Option<SearchResult> {
    type Register = Bits<u64, 64>;
    type Error = DecodeSearchResultError;

    fn encode(&self) -> Self::Register {
        match self {
            None => Bits::max(),
            Some(r) => {
                let mut register = Bits::default();
                let (kind, rest) = register.split_at_mut(2);
                let (score, rest) = rest.split_at_mut(16);
                let (height, rest) = rest.split_at_mut(7);
                let (action, signature) = rest.split_at_mut(<Action as Binary>::Register::WIDTH);

                kind.store(r.kind as u8);
                score.store(r.score);
                height.store(r.height);
                action.clone_from_bitslice(&r.action.encode());
                signature.clone_from_bitslice(&r.signature);

                register
            }
        }
    }

    fn decode(register: Self::Register) -> Result<Self, Self::Error> {
        if register == Bits::max() {
            Ok(None)
        } else {
            let (kind, rest) = register.split_at(2);
            let (score, rest) = rest.split_at(16);
            let (height, rest) = rest.split_at(7);
            let (action, signature) = rest.split_at(<Action as Binary>::Register::WIDTH);

            use SearchResultKind::*;
            Ok(Some(SearchResult {
                kind: [Lower, Upper, Exact]
                    .into_iter()
                    .nth(kind.load())
                    .ok_or(DecodeSearchResultError(register))?,
                score: score.load(),
                height: height.load::<u8>() as i8,
                action: Action::decode(action.into())
                    .map_err(|_| DecodeSearchResultError(register))?,
                signature: signature.into(),
            }))
        }
    }
}

/// Configuration for [`Minimax`].
#[derive(Debug, Display, Copy, Clone, Eq, PartialEq, Hash, Deserialize, Serialize)]
#[cfg_attr(test, derive(test_strategy::Arbitrary))]
#[display(fmt = "{}", "ron::ser::to_string(self).unwrap()")]
#[serde(deny_unknown_fields, rename = "config", default)]
pub struct MinimaxConfig {
    /// The maximum number of plies to search.
    ///
    /// This is an upper limit, the actual depth searched may be smaller.
    #[cfg_attr(test, strategy(0u8..=MinimaxConfig::default().max_depth))]
    pub max_depth: u8,

    /// The size of the transposition table in bytes.
    ///
    /// This is an upper limit, the actual memory allocation may be smaller.
    #[cfg_attr(test, strategy(8usize..=MinimaxConfig::default().table_size))]
    pub table_size: usize,
}

impl Default for MinimaxConfig {
    fn default() -> Self {
        #[cfg(test)]
        #[cfg(tarpaulin)]
        {
            Self {
                max_depth: 2,
                table_size: 1 << 8,
            }
        }

        #[cfg(test)]
        #[cfg(not(tarpaulin))]
        {
            Self {
                max_depth: 3,
                table_size: 1 << 16,
            }
        }

        #[cfg(not(test))]
        {
            Self {
                max_depth: 6,
                table_size: 1 << 32,
            }
        }
    }
}

/// The reason why parsing [`MinimaxConfig`] failed.
#[derive(Debug, Display, PartialEq, Error, From)]
#[display(fmt = "failed to parse minimax configuration")]
pub struct ParseMinimaxConfigError(ron::de::Error);

impl FromStr for MinimaxConfig {
    type Err = ParseMinimaxConfigError;

    fn from_str(s: &str) -> Result<Self, Self::Err> {
        Ok(ron::de::from_str(s)?)
    }
}

/// An implementation of [minimax].
///
/// [minimax]: https://en.wikipedia.org/wiki/Minimax
#[derive(Debug)]
pub struct Minimax<E: Eval + Send + Sync> {
    engine: E,
    config: MinimaxConfig,
    tt: Cache<Option<SearchResult>>,
}

impl<E: Eval + Send + Sync> Minimax<E> {
    /// Constructs [`Minimax`] with the default [`MinimaxConfig`].
    pub fn new(engine: E) -> Self {
        Self::with_config(engine, MinimaxConfig::default())
    }

    /// Constructs [`Minimax`] with the specified [`MinimaxConfig`].
    pub fn with_config(engine: E, config: MinimaxConfig) -> Self {
        let entry_size = <Option<SearchResult> as Binary>::Register::SIZE;
        let cache_size = (config.table_size / entry_size / 2 + 1).next_power_of_two();

        Minimax {
            engine,
            config,
            tt: Cache::new(cache_size),
        }
    }

    fn key_of(&self, game: &Game) -> (usize, Bits<u32, 24>) {
        let zobrist = game.position().signature();
        let signature = zobrist[40..].into();
        match self.tt.len().trailing_zeros() as usize {
            0 => (0, signature),
            w => (zobrist[..w].load(), signature),
        }
    }

    /// The [alpha-beta pruning] algorithm.
    ///
    /// [alpha-beta pruning]: https://en.wikipedia.org/wiki/Alpha%E2%80%93beta_pruning
    fn alpha_beta(&self, game: &Game, height: i8, alpha: i16, beta: i16) -> i16 {
        debug_assert!(alpha < beta, "{} < {}", alpha, beta);

        let (key, signature) = self.key_of(game);
        let (alpha, beta, score) = match self.tt.load(key) {
            Some(r) if r.height >= height && r.signature == signature => match r.kind {
                SearchResultKind::Lower => (alpha.max(r.score), beta, Some(r.score)),
                SearchResultKind::Upper => (alpha, beta.min(r.score), Some(r.score)),
                SearchResultKind::Exact => (r.score, r.score, Some(r.score)),
            },
            _ => (alpha, beta, None),
        };

        if alpha >= beta || height <= 0 || game.outcome().is_some() {
            return score.unwrap_or_else(|| self.engine.eval(game));
        }

        let cutoff = AtomicI16::new(alpha);

        let (action, score) = game
            .actions()
            .par_bridge()
            .map(|a| {
                let alpha = cutoff.load(Ordering::Relaxed);

                if alpha >= beta {
                    return None;
                }

                let mut game = game.clone();
                game.execute(a).expect("expected legal action");

                let score = self
                    .alpha_beta(
                        &game,
                        height - 1,
                        beta.saturating_neg(),
                        alpha.saturating_neg(),
                    )
                    .saturating_neg();

                cutoff.fetch_max(score, Ordering::Relaxed);

                Some((a, score))
            })
            .while_some()
            .max_by_key(|(_, s)| *s)
            .expect("expected at least one legal action");

        let kind = if score >= beta {
            SearchResultKind::Lower
        } else if score <= alpha {
            SearchResultKind::Upper
        } else {
            SearchResultKind::Exact
        };

        let result = SearchResult {
            kind,
            score,
            height,
            action,
            signature,
        };

        self.tt.update(key, |r| match r {
            Some(r) if (r.height, r.kind) > (height, kind) => None,
            _ => Some(result.into()),
        });

        score
    }

    /// The [mtd(f)] algorithm.
    ///
    /// [mtd(f)]: https://en.wikipedia.org/wiki/MTD(f)
    fn mtdf(&self, game: &Game, depth: i8, mut score: i16) -> i16 {
        let mut alpha = -i16::MAX;
        let mut beta = i16::MAX;
        while alpha < beta {
            let target = score.max(alpha + 1);
            score = self.alpha_beta(game, depth, target - 1, target);
            if score < target {
                beta = score;
            } else {
                alpha = score;
            }
        }

        score
    }
}

impl<E: Eval + Send + Sync> Search for Minimax<E> {
    fn search(&self, game: &Game) -> Option<Action> {
        let mut score = 0;
        for d in 1..=self.config.max_depth.min(i8::MAX as u8) as i8 {
            score = self.mtdf(game, d, score);
        }
        let (key, _) = self.key_of(game);
        self.tt.load(key).map(|r| r.action)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::{engine::Heuristic, MockEval, Outcome};
    use mockall::predicate::*;
    use test_strategy::proptest;

    fn minimax<E: Eval + Sync>(engine: &E, game: &Game, height: i8) -> i16 {
        if height == 0 || game.outcome().is_some() {
            return engine.eval(game);
        }

        game.actions()
            .par_bridge()
            .map(|a| {
                let mut game = game.clone();
                game.execute(a).unwrap();
                minimax(engine, &game, height - 1).saturating_neg()
            })
            .max()
            .unwrap()
    }

    #[proptest]
    fn decoding_encoded_search_result_is_an_identity(r: Option<SearchResult>) {
        assert_eq!(Option::decode(r.encode()), Ok(r));
    }

    #[proptest]
    fn config_deserializes_missing_fields_to_default() {
        assert_eq!("config()".parse(), Ok(MinimaxConfig::default()));
    }

    #[proptest]
    fn parsing_printed_config_is_an_identity(c: MinimaxConfig) {
        assert_eq!(c.to_string().parse(), Ok(c));
    }

    #[proptest]
    fn table_size_is_an_upper_limit(c: MinimaxConfig) {
        let strategy = Minimax::with_config(MockEval::new(), c);
        assert!(strategy.tt.len() * 8 <= c.table_size);
    }

    #[proptest]
    fn table_size_is_exact_if_power_of_two(#[strategy(3usize..=10)] w: usize) {
        let strategy = Minimax::with_config(
            MockEval::new(),
            MinimaxConfig {
                table_size: 1 << w,
                ..MinimaxConfig::default()
            },
        );

        assert_eq!(strategy.tt.len() * 8, 1 << w);
    }

    #[proptest]
    #[should_panic]
    #[cfg(debug_assertions)]
    fn alpha_beta_panics_if_alpha_not_smaller_than_beta(g: Game, a: i16, b: i16) {
        Minimax::new(MockEval::new()).alpha_beta(&g, 0, a.max(b), a.min(b));
    }

    #[proptest]
    fn alpha_beta_returns_none_if_depth_is_zero(g: Game, s: i16) {
        let mut engine = MockEval::new();
        engine
            .expect_eval()
            .once()
            .with(eq(g.clone()))
            .return_const(s);

        let strategy = Minimax::new(engine);
        assert_eq!(strategy.alpha_beta(&g, 0, i16::MIN, i16::MAX), s);
    }

    #[proptest]
    fn alpha_beta_returns_none_if_game_has_ended(
        _o: Outcome,
        #[any(Some(#_o))] g: Game,
        d: i8,
        s: i16,
    ) {
        let mut engine = MockEval::new();
        engine
            .expect_eval()
            .once()
            .with(eq(g.clone()))
            .return_const(s);

        let strategy = Minimax::new(engine);
        assert_eq!(strategy.alpha_beta(&g, d, i16::MIN, i16::MAX), s);
    }

    #[proptest]
    fn alpha_beta_returns_best_score(c: MinimaxConfig, g: Game) {
        let depth = c.max_depth.try_into()?;

        assert_eq!(
            minimax(&Heuristic::new(), &g, depth),
            Minimax::with_config(Heuristic::new(), c).alpha_beta(&g, depth, i16::MIN, i16::MAX),
        );
    }

    #[proptest]
    fn alpha_beta_does_not_depend_on_table_size(
        #[strategy(0usize..65536)] a: usize,
        #[strategy(0usize..65536)] b: usize,
        c: MinimaxConfig,
        g: Game,
    ) {
        let a = Minimax::with_config(Heuristic::new(), MinimaxConfig { table_size: a, ..c });
        let b = Minimax::with_config(Heuristic::new(), MinimaxConfig { table_size: b, ..c });

        let depth = c.max_depth.try_into()?;

        assert_eq!(
            a.alpha_beta(&g, depth, i16::MIN, i16::MAX),
            b.alpha_beta(&g, depth, i16::MIN, i16::MAX)
        );
    }

    #[proptest]
    fn mtdf_returns_best_score(c: MinimaxConfig, g: Game) {
        let depth = c.max_depth.try_into()?;

        assert_eq!(
            minimax(&Heuristic::new(), &g, depth),
            Minimax::with_config(Heuristic::new(), c).mtdf(&g, depth, 0),
        );
    }

    #[proptest]
    fn mtdf_does_not_depend_on_initial_guess(c: MinimaxConfig, g: Game, s: i16) {
        let a = Minimax::with_config(Heuristic::new(), c);
        let b = Minimax::with_config(Heuristic::new(), c);

        let depth = c.max_depth.try_into()?;

        assert_eq!(a.mtdf(&g, depth, s), b.mtdf(&g, depth, 0));
    }

    #[proptest]
    fn mtdf_is_equivalent_to_alphabeta(c: MinimaxConfig, g: Game) {
        let a = Minimax::with_config(Heuristic::new(), c);
        let b = Minimax::with_config(Heuristic::new(), c);

        let depth = c.max_depth.try_into()?;

        assert_eq!(
            a.mtdf(&g, depth, 0),
            b.alpha_beta(&g, depth, i16::MIN, i16::MAX),
        );
    }

    #[proptest]
    fn search_finds_the_best_action(c: MinimaxConfig, g: Game) {
        let strategy = Minimax::with_config(Heuristic::new(), c);
        let (key, _) = strategy.key_of(&g);
        assert_eq!(strategy.search(&g), strategy.tt.load(key).map(|r| r.action));
    }
}
