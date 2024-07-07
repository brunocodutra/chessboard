use crate::chess::{Color, Mirror};
use crate::nnue::{Accumulator, Feature, Nnue};
use crate::util::AlignTo64;

/// An accumulator for the psqt transformer.
#[derive(Debug, Clone, Eq, PartialEq, Hash)]
#[cfg_attr(test, derive(test_strategy::Arbitrary))]
pub struct Material(
    #[cfg_attr(test, map(|vs: [[i8; Self::LEN]; 2]| AlignTo64(vs.map(|v| v.map(i32::from)))))]
    AlignTo64<[[i32; Self::LEN]; 2]>,
);

impl Default for Material {
    #[inline(always)]
    fn default() -> Self {
        Material(AlignTo64([Nnue::psqt().fresh(); 2]))
    }
}

impl Accumulator for Material {
    const LEN: usize = 8;

    #[inline(always)]
    fn add(&mut self, white: Feature, black: Feature) {
        Nnue::psqt().add(white, &mut self.0[0]);
        Nnue::psqt().add(black, &mut self.0[1]);
    }

    #[inline(always)]
    fn remove(&mut self, white: Feature, black: Feature) {
        Nnue::psqt().remove(white, &mut self.0[0]);
        Nnue::psqt().remove(black, &mut self.0[1]);
    }

    #[inline(always)]
    fn replace(&mut self, white: [Feature; 2], black: [Feature; 2]) {
        Nnue::psqt().replace(white, &mut self.0[0]);
        Nnue::psqt().replace(black, &mut self.0[1]);
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
    fn material_evaluation_is_antisymmetric(e: Material, c: Color, #[strategy(..8usize)] p: usize) {
        assert_eq!(e.evaluate(c, p), -e.evaluate(!c, p));
    }

    #[proptest]
    fn remove_reverses_add(e: Material, w: Feature, b: Feature) {
        let mut f = e.clone();
        f.add(w, b);
        f.remove(w, b);
        assert_eq!(e, f);
    }
}
