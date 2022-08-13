use crate::{Binary, Bits, Cache, Move, Position, Pv, Register, Zobrist};
use bitvec::field::BitField;
use derive_more::{Display, Error};
use std::ops::RangeInclusive;

#[cfg(test)]
use proptest::{collection::*, prelude::*};

#[derive(Debug, Copy, Clone, Eq, PartialEq, Ord, PartialOrd, Hash)]
#[cfg_attr(test, derive(test_strategy::Arbitrary))]
enum TranspositionKind {
    Lower,
    Upper,
}

/// A partial search result.
#[derive(Debug, Copy, Clone, Eq, PartialEq, Hash)]
#[cfg_attr(test, derive(test_strategy::Arbitrary))]
pub struct Transposition {
    kind: TranspositionKind,
    score: i16,
    #[cfg_attr(test, strategy(Self::MIN_DRAFT..=Self::MAX_DRAFT))]
    draft: i8,
    best: Move,
}

impl Transposition {
    pub const MIN_DRAFT: i8 = (i8::MIN >> 1) + 32;
    pub const MAX_DRAFT: i8 = (i8::MAX >> 1) + 32;

    fn new(kind: TranspositionKind, score: i16, draft: i8, best: Move) -> Self {
        assert!(draft >= Self::MIN_DRAFT, "{} >= {}", draft, Self::MIN_DRAFT);
        assert!(draft <= Self::MAX_DRAFT, "{} <= {}", draft, Self::MAX_DRAFT);

        Transposition {
            kind,
            score,
            draft,
            best,
        }
    }

    /// Constructs a [`Transposition`] given a lower bound for the score, remaining draft, and best [`Move`].
    pub fn lower(score: i16, draft: i8, best: Move) -> Self {
        Transposition::new(TranspositionKind::Lower, score, draft, best)
    }

    /// Constructs a [`Transposition`] given an upper bound for the score, remaining draft, and best [`Move`].
    pub fn upper(score: i16, draft: i8, best: Move) -> Self {
        Transposition::new(TranspositionKind::Upper, score, draft, best)
    }

    /// Bounds for the exact score.
    pub fn bounds(&self) -> RangeInclusive<i16> {
        match self.kind {
            TranspositionKind::Lower => self.score..=i16::MAX,
            TranspositionKind::Upper => i16::MIN..=self.score,
        }
    }

    /// Remaining draft.
    pub fn draft(&self) -> i8 {
        self.draft
    }

    /// Partial score.
    pub fn score(&self) -> i16 {
        self.score
    }

    /// Best [`Move`] at this depth.
    pub fn best(&self) -> Move {
        self.best
    }
}

type Signature = Bits<u32, 25>;
type OptionalSignedTransposition = Option<(Transposition, Signature)>;
type OptionalSignedTranspositionRegister = <OptionalSignedTransposition as Binary>::Register;

/// The reason why decoding [`Transposition`] from binary failed.
#[derive(Debug, Display, Clone, Eq, PartialEq, Error)]
#[cfg_attr(test, derive(test_strategy::Arbitrary))]
#[display(fmt = "`{}` is not a valid search result", _0)]
pub struct DecodeTranspositionError(#[error(not(source))] OptionalSignedTranspositionRegister);

impl Binary for OptionalSignedTransposition {
    type Register = Bits<u64, 64>;
    type Error = DecodeTranspositionError;

    fn encode(&self) -> Self::Register {
        match self {
            None => Bits::default(),
            Some((t, sig)) => {
                let mut register = Bits::default();
                let (kind, rest) = register.split_at_mut(1);
                let (score, rest) = rest.split_at_mut(16);
                let (draft, rest) = rest.split_at_mut(7);
                let (best, rest) = rest.split_at_mut(<Move as Binary>::Register::WIDTH);

                kind.store(t.kind as u8);
                score.store(t.score);
                draft.store(t.draft - 32);
                best.clone_from_bitslice(&t.best.encode());
                rest.clone_from_bitslice(sig);

                debug_assert_ne!(register, Bits::default());

                register
            }
        }
    }

    fn decode(register: Self::Register) -> Result<Self, Self::Error> {
        if register == Bits::default() {
            Ok(None)
        } else {
            let (kind, rest) = register.split_at(1);
            let (score, rest) = rest.split_at(16);
            let (draft, rest) = rest.split_at(7);
            let (best, rest) = rest.split_at(<Move as Binary>::Register::WIDTH);

            use TranspositionKind::*;
            Ok(Some((
                Transposition {
                    kind: [Lower, Upper][kind.load::<usize>()],
                    score: score.load(),
                    draft: draft.load::<i8>() + 32,
                    best: Binary::decode(best.into())
                        .map_err(|_| DecodeTranspositionError(register))?,
                },
                rest.into(),
            )))
        }
    }
}

/// A cache for [`Transposition`]s.
#[derive(Debug)]
#[cfg_attr(test, derive(test_strategy::Arbitrary))]
pub struct TranspositionTable {
    #[cfg_attr(test, strategy(
        (1usize..=128, hash_map(any::<Position>(), any::<Transposition>(), 0..=32)).prop_map(|(cap, ts)| {
            let cache = Cache::new(cap);

            for (pos, t) in ts {
                let key = pos.zobrist();
                let idx = key.load::<usize>() & (cache.len() - 1);
                let sig = key[(Zobrist::WIDTH - Signature::WIDTH)..].into();
                cache.store(idx, Some((t, sig)).encode());
            }

            cache
        })
        .no_shrink()
    ))]
    cache: Cache<OptionalSignedTranspositionRegister>,
}

impl TranspositionTable {
    /// Constructs a [`TranspositionTable`] of at most `size` many bytes.
    ///
    /// The `size` specifies an upper bound, as long as the table is not empty.
    pub fn new(size: usize) -> Self {
        let entry_size = OptionalSignedTranspositionRegister::SIZE;
        let cache_size = (size / entry_size + 1).next_power_of_two() / 2;

        TranspositionTable {
            cache: Cache::new(cache_size.max(1)),
        }
    }

    /// The actual size of this [`TranspositionTable`] in bytes.
    pub fn size(&self) -> usize {
        self.capacity() * OptionalSignedTranspositionRegister::SIZE
    }

    /// The actual size of this [`TranspositionTable`] in number of entries.
    pub fn capacity(&self) -> usize {
        self.cache.len()
    }

    fn signature_of(&self, key: Zobrist) -> Signature {
        key[(Zobrist::WIDTH - Signature::WIDTH)..].into()
    }

    fn index_of(&self, key: Zobrist) -> usize {
        key.load::<usize>() & (self.capacity() - 1)
    }

    /// Loads the [`Transposition`] from the slot associated with `key`.
    pub fn get(&self, key: Zobrist) -> Option<Transposition> {
        let sig = self.signature_of(key);
        let register = self.cache.load(self.index_of(key));
        match Binary::decode(register).expect("expected valid encoding") {
            Some((t, s)) if s == sig => Some(t),
            _ => None,
        }
    }

    /// Stores a [`Transposition`] in the slot associated with `key`.
    ///
    /// In the slot if not empty, the [`Transposition`] with greater draft is chosen.
    pub fn set(&self, key: Zobrist, transposition: Transposition) {
        let sig = self.signature_of(key);
        self.cache.update(self.index_of(key), |r| {
            match Binary::decode(r).expect("expected valid encoding") {
                Some((t, _)) if t.draft() > transposition.draft() => None,
                _ => Some((transposition, sig)).encode().into(),
            }
        })
    }

    /// Clears the [`Transposition`] from the slot associated with `key`.
    pub fn clear(&self, key: Zobrist) {
        self.cache.store(self.index_of(key), None.encode())
    }

    /// An iterator for the [principal variation][`Pv`] from a starting [`Position`].
    pub fn pv(&self, pos: Position) -> Pv {
        Pv::new(self, pos)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use bitvec::view::BitView;
    use test_strategy::proptest;

    #[proptest]
    fn lower_constructs_lower_bound_transposition(
        s: i16,
        #[strategy(Transposition::MIN_DRAFT..=Transposition::MAX_DRAFT)] d: i8,
        m: Move,
    ) {
        assert_eq!(
            Transposition::lower(s, d, m),
            Transposition::new(TranspositionKind::Lower, s, d, m)
        );
    }

    #[proptest]
    fn upper_constructs_lower_bound_transposition(
        s: i16,
        #[strategy(Transposition::MIN_DRAFT..=Transposition::MAX_DRAFT)] d: i8,
        m: Move,
    ) {
        assert_eq!(
            Transposition::upper(s, d, m),
            Transposition::new(TranspositionKind::Upper, s, d, m)
        );
    }

    #[proptest]
    #[cfg(debug_assertions)]
    #[should_panic]
    fn transposition_panics_if_draft_grater_than_max(
        k: TranspositionKind,
        s: i16,
        #[strategy(Transposition::MAX_DRAFT + 1..)] d: i8,
        m: Move,
    ) {
        Transposition::new(k, s, d, m);
    }

    #[proptest]
    #[cfg(debug_assertions)]
    #[should_panic]
    fn transposition_panics_if_draft_lower_than_max(
        k: TranspositionKind,
        s: i16,
        #[strategy(..Transposition::MIN_DRAFT - 1)] d: i8,
        m: Move,
    ) {
        Transposition::new(k, s, d, m);
    }

    #[proptest]
    fn transposition_score_is_between_bounds(t: Transposition) {
        assert!(t.bounds().contains(&t.score()));
    }

    #[proptest]
    fn decoding_encoded_transposition_is_an_identity(t: OptionalSignedTransposition) {
        assert_eq!(Binary::decode(t.encode()), Ok(t));
    }

    #[proptest]
    fn size_returns_table_capacity_in_bytes(tt: TranspositionTable) {
        assert_eq!(
            tt.size(),
            tt.cache.len() * OptionalSignedTranspositionRegister::SIZE
        );
    }

    #[proptest]
    fn input_size_is_an_upper_limit(
        #[strategy(OptionalSignedTranspositionRegister::SIZE..=128)] s: usize,
    ) {
        assert!(TranspositionTable::new(s).size() <= s);
    }

    #[proptest]
    fn size_is_exact_if_input_is_power_of_two(
        #[strategy(OptionalSignedTranspositionRegister::SIZE..=128)] s: usize,
    ) {
        assert_eq!(
            TranspositionTable::new(s.next_power_of_two()).size(),
            s.next_power_of_two()
        );
    }

    #[proptest]
    fn capacity_returns_cache_len(tt: TranspositionTable) {
        assert_eq!(tt.capacity(), tt.cache.len());
    }

    #[proptest]
    fn get_returns_none_if_transposition_does_not_exist(tt: TranspositionTable, k: Zobrist) {
        tt.cache.store(tt.index_of(k), Bits::default());
        assert_eq!(tt.get(k), None);
    }

    #[proptest]
    fn get_returns_none_if_signature_does_not_match(
        tt: TranspositionTable,
        t: Transposition,
        k: Zobrist,
    ) {
        let sig = tt.signature_of((!k.load::<u64>()).view_bits().into());
        tt.cache.store(tt.index_of(k), Some((t, sig)).encode());
        assert_eq!(tt.get(k), None);
    }

    #[proptest]
    fn get_returns_some_if_transposition_exists(
        tt: TranspositionTable,
        t: Transposition,
        k: Zobrist,
    ) {
        let sig = tt.signature_of(k);
        tt.cache.store(tt.index_of(k), Some((t, sig)).encode());
        assert_eq!(tt.get(k), Some(t));
    }

    #[proptest]
    fn set_keeps_transposition_with_larger_draft(
        tt: TranspositionTable,
        t: Transposition,
        u: Transposition,
        k: Zobrist,
    ) {
        let sig = tt.signature_of(k);
        tt.cache.store(tt.index_of(k), Some((t, sig)).encode());
        tt.set(k, u);

        if t.draft() > u.draft() {
            assert_eq!(tt.get(k), Some(t));
        } else {
            assert_eq!(tt.get(k), Some(u));
        }
    }

    #[proptest]
    fn set_ignores_the_signature_mismatch(
        tt: TranspositionTable,
        t: Transposition,
        #[filter(#u.draft() > #t.draft())] u: Transposition,
        k: Zobrist,
    ) {
        let sig = tt.signature_of((!k.load::<u64>()).view_bits().into());
        tt.cache.store(tt.index_of(k), Some((t, sig)).encode());
        tt.set(k, u);
        assert_eq!(tt.get(k), Some(u));
    }

    #[proptest]
    fn set_stores_transposition_if_none_exists(
        tt: TranspositionTable,
        t: Transposition,
        k: Zobrist,
    ) {
        tt.cache.store(tt.index_of(k), Bits::default());
        tt.set(k, t);
        assert_eq!(tt.get(k), Some(t));
    }

    #[proptest]
    fn clear_erases_transposition(tt: TranspositionTable, k: Zobrist) {
        tt.clear(k);
        assert_eq!(tt.get(k), None);
    }
}
