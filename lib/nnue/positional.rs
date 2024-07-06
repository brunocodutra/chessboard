use crate::chess::{Color, Mirror};
use crate::nnue::{Accumulator, Feature, Nnue};
use crate::util::AlignTo64;

/// An accumulator for the feature transformer.
#[derive(Debug, Clone, Eq, PartialEq, Hash)]
#[cfg_attr(test, derive(test_strategy::Arbitrary))]
pub struct Positional(
    #[cfg_attr(test, map(|vs: [[i8; Self::LEN]; 2]| AlignTo64(vs.map(|v| v.map(i16::from)))))]
    AlignTo64<[[i16; Self::LEN]; 2]>,
);

impl Default for Positional {
    #[inline(always)]
    fn default() -> Self {
        Positional(AlignTo64([Nnue::ft().fresh(); 2]))
    }
}

impl Accumulator for Positional {
    const LEN: usize = 512;

    #[inline(always)]
    fn add(&mut self, white: Feature, black: Feature) {
        Nnue::ft().add(white, &mut self.0[0]);
        Nnue::ft().add(black, &mut self.0[1]);
    }

    #[inline(always)]
    fn remove(&mut self, white: Feature, black: Feature) {
        Nnue::ft().remove(white, &mut self.0[0]);
        Nnue::ft().remove(black, &mut self.0[1]);
    }

    #[inline(always)]
    fn evaluate(&self, turn: Color, phase: usize) -> i32 {
        Nnue::hidden(phase).forward([&self.0[turn as usize], &self.0[turn.mirror() as usize]]) / 16
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::nnue::Feature;
    use test_strategy::proptest;

    #[proptest]
    fn remove_reverses_add(e: Positional, w: Feature, b: Feature) {
        let mut f = e.clone();
        f.add(w, b);
        f.remove(w, b);
        assert_eq!(e, f);
    }
}
