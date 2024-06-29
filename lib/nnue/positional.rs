use crate::chess::{Color, Mirror};
use crate::nnue::{Accumulator, Nnue};
use crate::util::AlignTo64;

/// An accumulator for the feature transformer.
#[derive(Debug, Clone, Eq, PartialEq, Hash)]
#[cfg_attr(test, derive(test_strategy::Arbitrary))]
pub struct Positional(
    #[cfg_attr(test, map(|vs: [[i8; { Nnue::L1 }]; 2]| AlignTo64(vs.map(|v| v.map(i16::from)))))]
    AlignTo64<[[i16; Nnue::L1]; 2]>,
);

impl Default for Positional {
    #[inline(always)]
    fn default() -> Self {
        Positional(AlignTo64([[0; Nnue::L1]; 2]))
    }
}

impl Accumulator for Positional {
    #[inline(always)]
    fn refresh(&mut self, white: &[u16], black: &[u16]) {
        Nnue::ft().refresh(white, &mut self.0[0]);
        Nnue::ft().refresh(black, &mut self.0[1]);
    }

    #[inline(always)]
    fn add(&mut self, white: u16, black: u16) {
        Nnue::ft().add(white, &mut self.0[0]);
        Nnue::ft().add(black, &mut self.0[1]);
    }

    #[inline(always)]
    fn remove(&mut self, white: u16, black: u16) {
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
    use crate::{chess::Color, nnue::Feature};
    use test_strategy::proptest;

    #[proptest]
    fn refresh_resets_accumulator(mut a: Positional, mut b: Positional, f: Feature, c: Color) {
        a.refresh(&[f.index(c)], &[f.index(!c)]);
        b.refresh(&[f.index(c)], &[f.index(!c)]);
        assert_eq!(a, b);
    }

    #[proptest]
    fn remove_reverses_add(a: Positional, f: Feature, c: Color) {
        let mut b = a.clone();
        b.add(f.index(c), f.index(!c));
        b.remove(f.index(c), f.index(!c));
        assert_eq!(a, b);
    }
}
