use crate::nnue::{Accumulator, Nnue, Transformer, NNUE};
use crate::util::AlignTo64;
use std::ops::Deref;

/// An accumulator for the psqt transformer.
#[derive(Debug, Default, Clone, Eq, PartialEq, Hash)]
#[cfg_attr(test, derive(test_strategy::Arbitrary))]
pub struct Material(
    #[cfg_attr(test, map(|vs: [[i8; { Nnue::PHASES }]; 2]| AlignTo64(vs.map(|v| v.map(i32::from)))))]
     AlignTo64<[[i32; Nnue::PHASES]; 2]>,
);

impl Material {
    #[inline(always)]
    fn transformer(&self) -> impl Deref<Target = Transformer<i32, { Nnue::L0 }, { Nnue::PHASES }>> {
        unsafe { &NNUE.psqt }
    }
}

impl Accumulator for Material {
    #[inline(always)]
    fn mirror(&mut self) {
        self.0.reverse()
    }

    #[inline(always)]
    fn refresh(&mut self, us: &[u16], them: &[u16]) {
        self.transformer().refresh(us, &mut self.0[0]);
        self.transformer().refresh(them, &mut self.0[1]);
    }

    #[inline(always)]
    fn add(&mut self, us: u16, them: u16) {
        self.transformer().add(us, &mut self.0[0]);
        self.transformer().add(them, &mut self.0[1]);
    }

    #[inline(always)]
    fn remove(&mut self, us: u16, them: u16) {
        self.transformer().remove(us, &mut self.0[0]);
        self.transformer().remove(them, &mut self.0[1]);
    }

    #[inline(always)]
    fn evaluate(&self, phase: usize) -> i32 {
        (self.0[0][phase] - self.0[1][phase]) / 32
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::{chess::Color, nnue::Feature};
    use test_strategy::proptest;

    #[proptest]
    fn material_evaluation_is_symmetric(a: Material, #[strategy(..8usize)] phase: usize) {
        let mut mirrored = a.clone();
        mirrored.mirror();
        assert_eq!(a.evaluate(phase), -mirrored.evaluate(phase));
    }

    #[proptest]
    fn double_mirror_is_idempotent(a: Material) {
        let mut b = a.clone();
        b.mirror();
        b.mirror();
        assert_eq!(a, b);
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
