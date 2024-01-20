use crate::nnue::{Accumulator, Layer, Nnue, NNUE};
use crate::util::AlignTo64;
use std::mem::transmute;

/// An accumulator for the feature transformer.
#[derive(Debug, Clone, Eq, PartialEq, Hash)]
#[cfg_attr(test, derive(test_strategy::Arbitrary))]
pub struct Positional(
    #[cfg_attr(test, map(|vs: [[i8; { Nnue::L1 / 2 }]; 2]| AlignTo64(vs.map(|v| v.map(i16::from)))))]
     AlignTo64<[[i16; Nnue::L1 / 2]; 2]>,
);

impl Default for Positional {
    fn default() -> Self {
        Positional(AlignTo64([[0; Nnue::L1 / 2]; 2]))
    }
}

impl Accumulator for Positional {
    #[inline(always)]
    fn mirror(&mut self) {
        self.0.reverse()
    }

    #[inline(always)]
    fn refresh(&mut self, us: &[u16], them: &[u16]) {
        NNUE.ft.refresh(us, &mut self.0[0]);
        NNUE.ft.refresh(them, &mut self.0[1]);
    }

    #[inline(always)]
    fn add(&mut self, us: u16, them: u16) {
        NNUE.ft.add(us, &mut self.0[0]);
        NNUE.ft.add(them, &mut self.0[1]);
    }

    #[inline(always)]
    fn remove(&mut self, us: u16, them: u16) {
        NNUE.ft.remove(us, &mut self.0[0]);
        NNUE.ft.remove(them, &mut self.0[1]);
    }

    #[inline(always)]
    fn evaluate(&self, phase: usize) -> i32 {
        let l1: &AlignTo64<[i16; Nnue::L1]> = unsafe { transmute(&self.0) };
        NNUE.output[phase].forward(l1) / 16
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::{chess::Color, nnue::Feature};
    use test_strategy::proptest;

    #[proptest]
    fn double_mirror_is_idempotent(a: Positional) {
        let mut b = a.clone();
        b.mirror();
        b.mirror();
        assert_eq!(a, b);
    }

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
