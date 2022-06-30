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

/// Configuration for [`Negamax`].
#[derive(Debug, Display, Copy, Clone, Eq, PartialEq, Hash, Deserialize, Serialize)]
#[cfg_attr(test, derive(test_strategy::Arbitrary))]
#[display(fmt = "{}", "ron::ser::to_string(self).unwrap()")]
#[serde(deny_unknown_fields, rename = "config", default)]
pub struct NegamaxConfig {
    /// The maximum number of plies to search.
    ///
    /// This is an upper limit, the actual depth searched may be smaller.
    #[cfg_attr(test, strategy(0u8..=NegamaxConfig::default().max_depth))]
    pub max_depth: u8,

    /// The size of the transposition table in bytes.
    ///
    /// This is an upper limit, the actual memory allocation may be smaller.
    #[cfg_attr(test, strategy(8usize..=NegamaxConfig::default().table_size))]
    pub table_size: usize,
}

impl Default for NegamaxConfig {
    fn default() -> Self {
        #[cfg(test)]
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

/// The reason why parsing [`NegamaxConfig`] failed.
#[derive(Debug, Display, PartialEq, Error, From)]
#[display(fmt = "failed to parse negamax configuration")]
pub struct ParseNegamaxConfigError(ron::de::Error);

impl FromStr for NegamaxConfig {
    type Err = ParseNegamaxConfigError;

    fn from_str(s: &str) -> Result<Self, Self::Err> {
        Ok(ron::de::from_str(s)?)
    }
}

/// An implementation of [negamax].
///
/// [negamax]: https://en.wikipedia.org/wiki/Negamax
#[derive(Debug)]
pub struct Negamax<E: Eval + Send + Sync> {
    engine: E,
    config: NegamaxConfig,
    tt: Cache<Option<SearchResult>>,
}

impl<E: Eval + Send + Sync> Negamax<E> {
    /// Constructs [`Negamax`] with the default [`NegamaxConfig`].
    pub fn new(engine: E) -> Self {
        Self::with_config(engine, NegamaxConfig::default())
    }

    /// Constructs [`Negamax`] with the specified [`NegamaxConfig`].
    pub fn with_config(engine: E, config: NegamaxConfig) -> Self {
        let entry_size = <Option<SearchResult> as Binary>::Register::SIZE;
        let cache_size = (config.table_size / entry_size / 2 + 1).next_power_of_two();

        Negamax {
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

    fn negamax(&self, game: &Game, height: i8, alpha: i16, beta: i16) -> i16 {
        debug_assert!(alpha < beta);

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
                    .negamax(
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
}

impl<E: Eval + Send + Sync> Search for Negamax<E> {
    fn search(&self, game: &Game) -> Option<Action> {
        let depth = self.config.max_depth.min(i8::MAX as u8) as i8;
        self.negamax(game, depth, i16::MIN, i16::MAX);
        let (key, _) = self.key_of(game);
        self.tt.load(key).map(|r| {
            debug_assert_eq!(r.kind, SearchResultKind::Exact);
            r.action
        })
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
        assert_eq!("config()".parse(), Ok(NegamaxConfig::default()));
    }

    #[proptest]
    fn parsing_printed_config_is_an_identity(c: NegamaxConfig) {
        assert_eq!(c.to_string().parse(), Ok(c));
    }

    #[proptest]
    fn table_size_is_an_upper_limit(c: NegamaxConfig) {
        let strategy = Negamax::with_config(MockEval::new(), c);
        assert!(strategy.tt.len() * 8 <= c.table_size);
    }

    #[proptest]
    fn table_size_is_exact_if_power_of_two(#[strategy(3usize..=10)] w: usize) {
        let strategy = Negamax::with_config(
            MockEval::new(),
            NegamaxConfig {
                table_size: 1 << w,
                ..NegamaxConfig::default()
            },
        );

        assert_eq!(strategy.tt.len() * 8, 1 << w);
    }

    #[proptest]
    #[should_panic]
    #[cfg(debug_assertions)]
    fn negamax_panics_if_alpha_not_smaller_than_beta(g: Game, a: i16, b: i16) {
        Negamax::new(MockEval::new()).negamax(&g, 0, a.max(b), a.min(b));
    }

    #[proptest]
    fn negamax_returns_none_if_depth_is_zero(g: Game, s: i16) {
        let mut engine = MockEval::new();
        engine
            .expect_eval()
            .once()
            .with(eq(g.clone()))
            .return_const(s);

        let strategy = Negamax::new(engine);
        assert_eq!(strategy.negamax(&g, 0, i16::MIN, i16::MAX), s);
    }

    #[proptest]
    fn negamax_returns_none_if_game_has_ended(
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

        let strategy = Negamax::new(engine);
        assert_eq!(strategy.negamax(&g, d, i16::MIN, i16::MAX), s);
    }

    #[proptest]
    #[cfg(not(tarpaulin))]
    fn negamax_returns_best_score(c: NegamaxConfig, g: Game) {
        let depth = c.max_depth.try_into()?;

        assert_eq!(
            minimax(&Heuristic::new(), &g, depth),
            Negamax::with_config(Heuristic::new(), c).negamax(&g, depth, i16::MIN, i16::MAX),
        );
    }

    #[proptest]
    #[cfg(not(tarpaulin))]
    fn result_does_not_depend_on_table_size(
        #[strategy(0usize..65536)] a: usize,
        #[strategy(0usize..65536)] b: usize,
        c: NegamaxConfig,
        g: Game,
    ) {
        let a = Negamax::with_config(Heuristic::new(), NegamaxConfig { table_size: a, ..c });
        let b = Negamax::with_config(Heuristic::new(), NegamaxConfig { table_size: b, ..c });

        let depth = c.max_depth.try_into()?;

        assert_eq!(
            a.negamax(&g, depth, i16::MIN, i16::MAX),
            b.negamax(&g, depth, i16::MIN, i16::MAX)
        );
    }
}
