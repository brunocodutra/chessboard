use crate::chess::{Color, Mirror};
use crate::nnue::{Accumulator, Nnue};
use crate::util::AlignTo64;

/// An accumulator for the psqt transformer.
#[derive(Debug, Default, Clone, Eq, PartialEq, Hash)]
#[cfg_attr(test, derive(test_strategy::Arbitrary))]
pub struct Material(
    #[cfg_attr(test, map(|vs: [[i8; Nnue::PHASES]; 2]| AlignTo64(vs.map(|v| v.map(i32::from)))))]
    AlignTo64<[[i32; Nnue::PHASES]; 2]>,
);

impl Accumulator for Material {
    #[inline(always)]
    fn refresh(&mut self, white: &[u16], black: &[u16]) {
        Nnue::psqt().refresh(white, &mut self.0[0]);
        Nnue::psqt().refresh(black, &mut self.0[1]);
    }

    #[inline(always)]
    fn add(&mut self, white: u16, black: u16) {
        Nnue::psqt().add(white, &mut self.0[0]);
        Nnue::psqt().add(black, &mut self.0[1]);
    }

    #[inline(always)]
    fn remove(&mut self, white: u16, black: u16) {
        Nnue::psqt().remove(white, &mut self.0[0]);
        Nnue::psqt().remove(black, &mut self.0[1]);
    }

    #[inline(always)]
    fn evaluate(&self, turn: Color, phase: usize) -> i32 {
        (self.0[turn as usize][phase] - self.0[turn.mirror() as usize][phase]) / 32
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::nnue::Feature;
    use test_strategy::proptest;

    #[proptest]
    fn material_evaluation_is_symmetric(a: Material, c: Color, #[strategy(..8usize)] p: usize) {
        assert_eq!(a.evaluate(c, p), -a.evaluate(!c, p));
    }

    #[proptest]
    fn refresh_resets_accumulator(mut a: Material, mut b: Material, f: Feature, c: Color) {
        a.refresh(&[f.index(c)], &[f.index(!c)]);
        b.refresh(&[f.index(c)], &[f.index(!c)]);
        assert_eq!(a, b);
    }

    #[proptest]
    fn remove_reverses_add(a: Material, f: Feature, c: Color) {
        let mut b = a.clone();
        b.add(f.index(c), f.index(!c));
        b.remove(f.index(c), f.index(!c));
        assert_eq!(a, b);
    }
}
