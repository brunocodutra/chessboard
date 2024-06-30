use crate::util::AlignTo64;
use derive_more::Constructor;

/// The hidden layer.
#[derive(Debug, Copy, Clone, Eq, PartialEq, Hash, Constructor)]
#[cfg_attr(test, derive(test_strategy::Arbitrary))]
pub struct Hidden<const N: usize> {
    #[cfg_attr(test, map(|b: i8| i32::from(b)))]
    pub(super) bias: i32,
    pub(super) weight: AlignTo64<[[i8; N]; 2]>,
}

impl<const N: usize> Hidden<N> {
    #[doc(hidden)]
    #[cfg(target_feature = "avx2")]
    pub unsafe fn avx2(&self, input: [&[i16; N]; 2]) -> i32 {
        const { assert!(N % 128 == 0) };

        use std::{arch::x86_64::*, mem::transmute};

        let mut y = _mm256_setzero_si256();
        for (w, i) in self.weight.iter().zip(input) {
            debug_assert_eq!(w.as_ptr() as usize % 32, 0);
            debug_assert_eq!(i.as_ptr() as usize % 32, 0);

            for (a, x) in Iterator::zip(w.array_chunks::<128>(), i.array_chunks::<128>()) {
                let a = transmute::<&[i8; 128], &[[i8; 32]; 4]>(a);
                let x = transmute::<&[i16; 128], &[[i16; 32]; 4]>(x);

                let p0 = _mm256_load_si256(x[0][..16].as_ptr() as _);
                let q0 = _mm256_load_si256(x[0][16..].as_ptr() as _);
                let p1 = _mm256_load_si256(x[1][..16].as_ptr() as _);
                let q1 = _mm256_load_si256(x[1][16..].as_ptr() as _);
                let p2 = _mm256_load_si256(x[2][..16].as_ptr() as _);
                let q2 = _mm256_load_si256(x[2][16..].as_ptr() as _);
                let p3 = _mm256_load_si256(x[3][..16].as_ptr() as _);
                let q3 = _mm256_load_si256(x[3][16..].as_ptr() as _);

                let r = _mm256_packs_epi16(p0, q0);
                let r = _mm256_max_epi8(r, _mm256_setzero_si256());
                let r = _mm256_permute4x64_epi64(r, 0b11011000);
                let r = _mm256_maddubs_epi16(r, _mm256_load_si256(a[0].as_ptr() as _));
                let r = _mm256_madd_epi16(r, _mm256_set1_epi16(1));
                y = _mm256_add_epi32(y, r);

                let r = _mm256_packs_epi16(p1, q1);
                let r = _mm256_max_epi8(r, _mm256_setzero_si256());
                let r = _mm256_permute4x64_epi64(r, 0b11011000);
                let r = _mm256_maddubs_epi16(r, _mm256_load_si256(a[1].as_ptr() as _));
                let r = _mm256_madd_epi16(r, _mm256_set1_epi16(1));
                y = _mm256_add_epi32(y, r);

                let r = _mm256_packs_epi16(p2, q2);
                let r = _mm256_max_epi8(r, _mm256_setzero_si256());
                let r = _mm256_permute4x64_epi64(r, 0b11011000);
                let r = _mm256_maddubs_epi16(r, _mm256_load_si256(a[2].as_ptr() as _));
                let r = _mm256_madd_epi16(r, _mm256_set1_epi16(1));
                y = _mm256_add_epi32(y, r);

                let r = _mm256_packs_epi16(p3, q3);
                let r = _mm256_max_epi8(r, _mm256_setzero_si256());
                let r = _mm256_permute4x64_epi64(r, 0b11011000);
                let r = _mm256_maddubs_epi16(r, _mm256_load_si256(a[3].as_ptr() as _));
                let r = _mm256_madd_epi16(r, _mm256_set1_epi16(1));
                y = _mm256_add_epi32(y, r);
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
        self.bias + _mm_cvtsi128_si32(r)
    }

    #[doc(hidden)]
    #[cfg(all(target_feature = "sse4.1", target_feature = "ssse3"))]
    pub unsafe fn sse(&self, input: [&[i16; N]; 2]) -> i32 {
        const { assert!(N % 64 == 0) };

        use std::{arch::x86_64::*, mem::transmute};

        let mut y = _mm_setzero_si128();
        for (w, i) in self.weight.iter().zip(input) {
            debug_assert_eq!(w.as_ptr() as usize % 16, 0);
            debug_assert_eq!(i.as_ptr() as usize % 16, 0);

            for (a, x) in Iterator::zip(w.array_chunks::<64>(), i.array_chunks::<64>()) {
                let a = transmute::<&[i8; 64], &[[i8; 16]; 4]>(a);
                let x = transmute::<&[i16; 64], &[[i16; 16]; 4]>(x);

                let p0 = _mm_load_si128(x[0][..8].as_ptr() as _);
                let q0 = _mm_load_si128(x[0][8..].as_ptr() as _);
                let p1 = _mm_load_si128(x[1][..8].as_ptr() as _);
                let q1 = _mm_load_si128(x[1][8..].as_ptr() as _);
                let p2 = _mm_load_si128(x[2][..8].as_ptr() as _);
                let q2 = _mm_load_si128(x[2][8..].as_ptr() as _);
                let p3 = _mm_load_si128(x[3][..8].as_ptr() as _);
                let q3 = _mm_load_si128(x[3][8..].as_ptr() as _);

                let r = _mm_packs_epi16(p0, q0);
                let r = _mm_max_epi8(r, _mm_setzero_si128());
                let r = _mm_maddubs_epi16(r, _mm_load_si128(a[0].as_ptr() as _));
                let r = _mm_madd_epi16(r, _mm_set1_epi16(1));
                y = _mm_add_epi32(y, r);

                let r = _mm_packs_epi16(p1, q1);
                let r = _mm_max_epi8(r, _mm_setzero_si128());
                let r = _mm_maddubs_epi16(r, _mm_load_si128(a[1].as_ptr() as _));
                let r = _mm_madd_epi16(r, _mm_set1_epi16(1));
                y = _mm_add_epi32(y, r);

                let r = _mm_packs_epi16(p2, q2);
                let r = _mm_max_epi8(r, _mm_setzero_si128());
                let r = _mm_maddubs_epi16(r, _mm_load_si128(a[2].as_ptr() as _));
                let r = _mm_madd_epi16(r, _mm_set1_epi16(1));
                y = _mm_add_epi32(y, r);

                let r = _mm_packs_epi16(p3, q3);
                let r = _mm_max_epi8(r, _mm_setzero_si128());
                let r = _mm_maddubs_epi16(r, _mm_load_si128(a[3].as_ptr() as _));
                let r = _mm_madd_epi16(r, _mm_set1_epi16(1));
                y = _mm_add_epi32(y, r);
            }
        }

        // https://stackoverflow.com/a/35270026
        let r = _mm_shuffle_epi32(y, _MM_SHUFFLE(1, 0, 3, 2));
        let s = _mm_add_epi32(r, y);
        let r = _mm_shufflelo_epi16(s, _MM_SHUFFLE(1, 0, 3, 2));
        let s = _mm_add_epi32(r, s);
        self.bias + _mm_cvtsi128_si32(s)
    }

    #[doc(hidden)]
    pub fn scalar(&self, input: [&[i16; N]; 2]) -> i32 {
        let mut y = self.bias;
        for (w, i) in self.weight.iter().zip(input) {
            for (&a, &x) in Iterator::zip(w.iter(), i.iter()) {
                y += a as i32 * x.clamp(0, i8::MAX as _) as i32;
            }
        }

        y
    }
}

impl<const N: usize> Hidden<N> {
    /// Transforms the accumulator.
    pub fn forward(&self, input: [&[i16; N]; 2]) -> i32 {
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
    fn uses_avx(o: Hidden<128>, i: AlignTo64<[[i16; 128]; 2]>) {
        assert_eq!(unsafe { o.avx2([&i[0], &i[1]]) }, o.scalar([&i[0], &i[1]]));
    }

    #[cfg(all(target_feature = "sse4.1", target_feature = "ssse3"))]
    #[proptest]
    fn uses_sse(o: Hidden<128>, i: AlignTo64<[[i16; 128]; 2]>) {
        assert_eq!(unsafe { o.sse([&i[0], &i[1]]) }, o.scalar([&i[0], &i[1]]));
    }

    #[proptest]
    fn clips_and_multiplies_by_weight_and_adds_bias(o: Hidden<128>, i: AlignTo64<[[i16; 128]; 2]>) {
        assert_eq!(
            o.forward([&i[0], &i[1]]),
            o.bias
                + Iterator::zip(o.weight.iter().flatten(), i.iter().flatten())
                    .map(|(&a, &x)| a as i32 * x.clamp(0, i8::MAX as _) as i32)
                    .sum::<i32>()
        );
    }
}
