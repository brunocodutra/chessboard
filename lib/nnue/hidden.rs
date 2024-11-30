use crate::util::AlignTo64;
use std::ops::Shl;

/// The hidden layer.
#[derive(Debug, Clone, Eq, PartialEq, Hash)]
#[cfg_attr(test, derive(test_strategy::Arbitrary))]
pub struct Hidden<const N: usize> {
    #[cfg_attr(test, map(|b: i8| i32::from(b)))]
    pub(super) bias: i32,
    pub(super) weight: AlignTo64<[[i8; N]; 2]>,
}

impl<const N: usize> Hidden<N> {
    #[doc(hidden)]
    #[inline(always)]
    #[cfg(target_feature = "avx2")]
    pub unsafe fn avx2(&self, us: &[i16; N], them: &[i16; N]) -> i32 {
        const { assert!(N % 128 == 0) }

        use std::{arch::x86_64::*, mem::transmute};

        #[inline(always)]
        unsafe fn crelu(p: __m256i, q: __m256i) -> __m256i {
            _mm256_packus_epi16(p, q)
        }

        #[inline(always)]
        unsafe fn square(r: __m256i) -> __m256i {
            let p = _mm256_unpacklo_epi8(r, _mm256_setzero_si256());
            let q = _mm256_unpackhi_epi8(r, _mm256_setzero_si256());
            let p = _mm256_slli_epi16(p, 3);
            let q = _mm256_slli_epi16(q, 3);
            let p = _mm256_mulhrs_epi16(p, p);
            let q = _mm256_mulhrs_epi16(q, q);
            let r = _mm256_packus_epi16(p, q);
            _mm256_permute4x64_epi64(r, _MM_SHUFFLE(3, 1, 2, 0))
        }

        #[inline(always)]
        unsafe fn dot(p: __m256i, q: __m256i) -> __m256i {
            _mm256_madd_epi16(_mm256_maddubs_epi16(p, q), _mm256_set1_epi16(1))
        }

        let mut y = _mm256_setr_epi32(self.bias, 0, 0, 0, 0, 0, 0, 0);

        for (w, i) in self.weight.iter().zip([us, them]) {
            debug_assert_eq!(w.as_ptr() as usize % 32, 0);
            debug_assert_eq!(i.as_ptr() as usize % 32, 0);

            for (a, x) in Iterator::zip(w.array_chunks::<128>(), i.array_chunks::<128>()) {
                let a = transmute::<&[i8; 128], &[__m256i; 4]>(a);
                let x = transmute::<&[i16; 128], &[[__m256i; 2]; 4]>(x);

                y = _mm256_add_epi32(y, dot(square(crelu(x[0][0], x[0][1])), a[0]));
                y = _mm256_add_epi32(y, dot(square(crelu(x[1][0], x[1][1])), a[1]));
                y = _mm256_add_epi32(y, dot(square(crelu(x[2][0], x[2][1])), a[2]));
                y = _mm256_add_epi32(y, dot(square(crelu(x[3][0], x[3][1])), a[3]));
            }
        }

        // https://stackoverflow.com/a/60109639
        let r = _mm256_castsi256_si128(y);
        let s = _mm256_extracti128_si256(y, 1);
        let r = _mm_add_epi32(r, s);
        let s = _mm_unpackhi_epi64(r, r);
        let r = _mm_add_epi32(r, s);
        let s = _mm_shuffle_epi32(r, _MM_SHUFFLE(2, 3, 0, 1));
        let r = _mm_add_epi32(r, s);
        _mm_extract_epi32(r, 0)
    }

    #[doc(hidden)]
    #[inline(always)]
    #[cfg(target_feature = "ssse3")]
    pub unsafe fn sse(&self, us: &[i16; N], them: &[i16; N]) -> i32 {
        const { assert!(N % 64 == 0) }

        use std::{arch::x86_64::*, mem::transmute};

        #[inline(always)]
        unsafe fn crelu(p: __m128i, q: __m128i) -> __m128i {
            _mm_packus_epi16(p, q)
        }

        #[inline(always)]
        unsafe fn square(r: __m128i) -> __m128i {
            let p = _mm_unpacklo_epi8(r, _mm_setzero_si128());
            let q = _mm_unpackhi_epi8(r, _mm_setzero_si128());
            let p = _mm_slli_epi16(p, 3);
            let q = _mm_slli_epi16(q, 3);
            let p = _mm_mulhrs_epi16(p, p);
            let q = _mm_mulhrs_epi16(q, q);
            _mm_packus_epi16(p, q)
        }

        #[inline(always)]
        unsafe fn dot(p: __m128i, q: __m128i) -> __m128i {
            _mm_madd_epi16(_mm_maddubs_epi16(p, q), _mm_set1_epi16(1))
        }

        let mut y = _mm_setr_epi32(self.bias, 0, 0, 0);

        for (w, i) in self.weight.iter().zip([us, them]) {
            debug_assert_eq!(w.as_ptr() as usize % 16, 0);
            debug_assert_eq!(i.as_ptr() as usize % 16, 0);

            for (a, x) in Iterator::zip(w.array_chunks::<64>(), i.array_chunks::<64>()) {
                let a = transmute::<&[i8; 64], &[__m128i; 4]>(a);
                let x = transmute::<&[i16; 64], &[[__m128i; 2]; 4]>(x);

                y = _mm_add_epi32(y, dot(square(crelu(x[0][0], x[0][1])), a[0]));
                y = _mm_add_epi32(y, dot(square(crelu(x[1][0], x[1][1])), a[1]));
                y = _mm_add_epi32(y, dot(square(crelu(x[2][0], x[2][1])), a[2]));
                y = _mm_add_epi32(y, dot(square(crelu(x[3][0], x[3][1])), a[3]));
            }
        }

        // https://stackoverflow.com/a/35270026
        let r = _mm_shuffle_epi32(y, _MM_SHUFFLE(1, 0, 3, 2));
        let s = _mm_add_epi32(r, y);
        let r = _mm_shufflelo_epi16(s, _MM_SHUFFLE(1, 0, 3, 2));
        let s = _mm_add_epi32(r, s);
        _mm_cvtsi128_si32(s)
    }

    #[doc(hidden)]
    #[inline(always)]
    pub fn scalar(&self, us: &[i16; N], them: &[i16; N]) -> i32 {
        let mut y = self.bias;
        for (w, i) in self.weight.iter().zip([us, them]) {
            for (&a, &x) in Iterator::zip(w.iter(), i.iter()) {
                y += a as i32 * (((x as i32).clamp(0, 255).shl(3i32).pow(2) + 16384) >> 15);
            }
        }

        y
    }
}

impl<const N: usize> Hidden<N> {
    /// Transforms the accumulator.
    #[inline(always)]
    pub fn forward(&self, us: &[i16; N], them: &[i16; N]) -> i32 {
        #[cfg(target_feature = "avx2")]
        unsafe {
            self.avx2(us, them)
        }

        #[cfg(not(target_feature = "avx2"))]
        #[cfg(target_feature = "ssse3")]
        unsafe {
            self.sse(us, them)
        }

        #[cfg(not(target_feature = "avx2"))]
        #[cfg(not(target_feature = "ssse3"))]
        self.scalar(us, them)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use test_strategy::proptest;

    #[cfg(target_feature = "avx2")]
    #[proptest]
    fn uses_avx(o: Hidden<128>, i: AlignTo64<[[i16; 128]; 2]>) {
        assert_eq!(unsafe { o.avx2(&i[0], &i[1]) }, o.scalar(&i[0], &i[1]));
    }

    #[cfg(target_feature = "ssse3")]
    #[proptest]
    fn uses_sse(o: Hidden<128>, i: AlignTo64<[[i16; 128]; 2]>) {
        assert_eq!(unsafe { o.sse(&i[0], &i[1]) }, o.scalar(&i[0], &i[1]));
    }
}
