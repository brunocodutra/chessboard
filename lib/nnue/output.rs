use crate::{nnue::Layer, util::AlignTo64};
use derive_more::Constructor;

/// The output layer.
#[derive(Debug, Copy, Clone, Eq, PartialEq, Hash, Constructor)]
#[cfg_attr(test, derive(test_strategy::Arbitrary))]
pub struct Output<const N: usize> {
    #[cfg_attr(test, map(|b: i8| i32::from(b)))]
    pub(super) bias: i32,
    pub(super) weight: AlignTo64<[i8; N]>,
}

impl<const N: usize> Output<N> {
    #[doc(hidden)]
    #[cfg(target_feature = "avx2")]
    pub unsafe fn avx2(&self, input: &AlignTo64<[i16; N]>) -> i32 {
        #[cfg(target_arch = "x86_64")]
        use std::arch::x86_64::*;

        #[cfg(target_arch = "x86")]
        use std::arch::x86::*;

        debug_assert!(N % 32 == 0);
        let a = self.weight.array_chunks::<32>();
        let x = input.array_chunks::<32>();
        let mut y = _mm256_setzero_si256();

        for (a, x) in Iterator::zip(a, x) {
            debug_assert_eq!(x[..16].as_ptr() as usize % 32, 0);
            let p = _mm256_load_si256(x[..16].as_ptr() as _);
            debug_assert_eq!(x[16..].as_ptr() as usize % 32, 0);
            let q = _mm256_load_si256(x[16..].as_ptr() as _);
            let r = _mm256_packs_epi16(p, q);
            let s = _mm256_max_epi8(r, _mm256_setzero_si256());
            let t = _mm256_permute4x64_epi64(s, 0b11011000);
            debug_assert_eq!(a.as_ptr() as usize % 32, 0);
            let u = _mm256_maddubs_epi16(t, _mm256_load_si256(a.as_ptr() as _));
            let v = _mm256_madd_epi16(u, _mm256_set1_epi16(1));
            y = _mm256_add_epi32(y, v);
        }

        // https://stackoverflow.com/a/60109639
        let p = _mm256_castsi256_si128(y);
        let q = _mm256_extracti128_si256(y, 1);
        let r = _mm_add_epi32(p, q);
        let s = _mm_unpackhi_epi64(r, r);
        let t = _mm_add_epi32(s, r);
        let u = _mm_shuffle_epi32(t, _MM_SHUFFLE(2, 3, 0, 1));
        let v = _mm_add_epi32(t, u);
        self.bias + _mm_cvtsi128_si32(v)
    }

    #[doc(hidden)]
    #[cfg(all(target_feature = "sse4.1", target_feature = "ssse3"))]
    pub unsafe fn sse(&self, input: &AlignTo64<[i16; N]>) -> i32 {
        #[cfg(target_arch = "x86_64")]
        use std::arch::x86_64::*;

        #[cfg(target_arch = "x86")]
        use std::arch::x86::*;

        debug_assert!(N % 16 == 0);
        let a = self.weight.array_chunks::<16>();
        let x = input.array_chunks::<16>();
        let mut y = _mm_setzero_si128();

        for (a, x) in Iterator::zip(a, x) {
            debug_assert_eq!(x[..8].as_ptr() as usize % 16, 0);
            let p = _mm_load_si128(x[..8].as_ptr() as _);
            debug_assert_eq!(x[8..].as_ptr() as usize % 16, 0);
            let q = _mm_load_si128(x[8..].as_ptr() as _);
            let r = _mm_packs_epi16(p, q);
            let s = _mm_max_epi8(r, _mm_setzero_si128());
            debug_assert_eq!(a.as_ptr() as usize % 16, 0);
            let t = _mm_maddubs_epi16(s, _mm_load_si128(a.as_ptr() as _));
            let u = _mm_madd_epi16(t, _mm_set1_epi16(1));
            y = _mm_add_epi32(y, u);
        }

        // https://stackoverflow.com/a/35270026
        let p = _mm_shuffle_epi32(y, _MM_SHUFFLE(1, 0, 3, 2));
        let q = _mm_add_epi32(p, y);
        let r = _mm_shufflelo_epi16(q, _MM_SHUFFLE(1, 0, 3, 2));
        let s = _mm_add_epi32(q, r);
        self.bias + _mm_cvtsi128_si32(s)
    }

    #[doc(hidden)]
    pub fn scalar(&self, input: &[i16; N]) -> i32 {
        let mut y = self.bias;
        for (&a, &x) in self.weight.iter().zip(input) {
            y += a as i32 * x.clamp(0, i8::MAX as _) as i32;
        }

        y
    }
}

impl<const N: usize> Layer<AlignTo64<[i16; N]>> for Output<N> {
    type Output = i32;

    fn forward(&self, input: &AlignTo64<[i16; N]>) -> Self::Output {
        #[cfg(target_feature = "avx2")]
        unsafe {
            self.avx2(input)
        }

        #[cfg(not(target_feature = "avx2"))]
        #[cfg(all(target_feature = "sse4.1", target_feature = "ssse3"))]
        unsafe {
            self.sse(input)
        }

        #[cfg(not(target_feature = "avx2"))]
        #[cfg(not(all(target_feature = "sse4.1", target_feature = "ssse3")))]
        self.scalar(input)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use test_strategy::proptest;

    #[cfg(target_feature = "avx2")]
    #[proptest]
    fn output_uses_avx(o: Output<64>, i: AlignTo64<[i16; 64]>) {
        assert_eq!(unsafe { o.avx2(&i) }, o.scalar(&i));
    }

    #[cfg(all(target_feature = "sse4.1", target_feature = "ssse3"))]
    #[proptest]
    fn output_uses_sse(o: Output<64>, i: AlignTo64<[i16; 64]>) {
        assert_eq!(unsafe { o.sse(&i) }, o.scalar(&i));
    }

    #[proptest]
    fn output_clips_and_multiplies_by_weight_and_adds_bias(o: Output<64>, i: AlignTo64<[i16; 64]>) {
        assert_eq!(
            o.forward(&i),
            o.bias
                + o.weight
                    .iter()
                    .zip(i)
                    .map(|(&a, x)| a as i32 * x.clamp(0, i8::MAX as _) as i32)
                    .sum::<i32>()
        );
    }
}
