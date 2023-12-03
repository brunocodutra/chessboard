use crate::util::Assume;
use std::mem::transmute;
use std::ops::{AddAssign, SubAssign};
use std::simd::{i32x4, i32x8, i8x16, i8x32, prelude::*, simd_swizzle};

#[cfg(target_arch = "x86_64")]
#[cfg(any(
    target_feature = "avx2",
    all(target_feature = "ssse3", target_feature = "sse2")
))]
use std::arch::x86_64::*;

#[cfg(target_arch = "x86")]
#[cfg(any(
    target_feature = "avx2",
    all(target_feature = "ssse3", target_feature = "sse2")
))]
use std::arch::x86::*;

/// A trait for types that implement affine transformations.
pub trait Axpy<A: ?Sized, X: ?Sized> {
    /// Computes `self += a * x`.
    fn axpy(&mut self, a: &A, x: &X);
}

impl<const I: usize> Axpy<[i8; I], [i8; I]> for i32 {
    fn axpy(&mut self, a: &[i8; I], x: &[i8; I]) {
        let mut a = a.array_chunks::<32>();
        let mut x = x.array_chunks::<32>();
        let mut y = [0; 8];

        for (a, x) in Iterator::zip(&mut a, &mut x) {
            y.axpy(a, x);
        }

        *self += i32x8::from_array(y).reduce_sum();

        let mut a = a.remainder().array_chunks::<16>();
        let mut x = x.remainder().array_chunks::<16>();
        let mut y = [0; 4];

        for (a, x) in Iterator::zip(&mut a, &mut x) {
            y.axpy(a, x);
        }

        *self += i32x4::from_array(y).reduce_sum();

        for (a, x) in a.remainder().iter().zip(x.remainder()) {
            *self += *x as i32 * *a as i32;
        }
    }
}

impl Axpy<[i8; 32], [i8; 32]> for [i32; 8] {
    fn axpy(&mut self, a: &[i8; 32], x: &[i8; 32]) {
        let (y, a, x) = (i32x8::from(*self), i8x32::from(*a), i8x32::from(*x));

        #[cfg(any(target_arch = "x86", target_arch = "x86_64"))]
        #[cfg(target_feature = "avx2")]
        let (y, a, x) = (__m256i::from(y), __m256i::from(a), __m256i::from(x));

        let mut y = y;
        y.axpy(&a, &x);

        #[cfg(any(target_arch = "x86", target_arch = "x86_64"))]
        #[cfg(target_feature = "avx2")]
        let y = i32x8::from(y);

        *self = y.to_array();
    }
}

impl Axpy<i8x32, i8x32> for i32x8 {
    fn axpy(&mut self, a: &i8x32, x: &i8x32) {
        let [ah, al] = unsafe { transmute::<_, &[[i8; 16]; 2]>(a) };
        let [xh, xl] = unsafe { transmute::<_, &[[i8; 16]; 2]>(x) };
        let [yh, yl] = unsafe { transmute::<_, &mut [[i32; 4]; 2]>(self) };

        yh.axpy(ah, xh);
        yl.axpy(al, xl);
    }
}

#[cfg(any(target_arch = "x86", target_arch = "x86_64"))]
#[cfg(target_feature = "avx2")]
impl Axpy<__m256i, __m256i> for __m256i {
    fn axpy(&mut self, a: &__m256i, x: &__m256i) {
        unsafe {
            let p = _mm256_maddubs_epi16(*x, *a);
            let q = _mm256_madd_epi16(p, _mm256_set1_epi16(1));
            *self = _mm256_add_epi32(*self, q);
        }
    }
}

impl Axpy<[i8; 16], [i8; 16]> for [i32; 4] {
    fn axpy(&mut self, a: &[i8; 16], x: &[i8; 16]) {
        let (y, a, x) = (i32x4::from(*self), i8x16::from(*a), i8x16::from(*x));

        #[cfg(any(target_arch = "x86", target_arch = "x86_64"))]
        #[cfg(all(target_feature = "ssse3", target_feature = "sse2"))]
        let (y, a, x) = (__m128i::from(y), __m128i::from(a), __m128i::from(x));

        let mut y = y;
        y.axpy(&a, &x);

        #[cfg(any(target_arch = "x86", target_arch = "x86_64"))]
        #[cfg(all(target_feature = "ssse3", target_feature = "sse2"))]
        let y = i32x4::from(y);

        *self = y.to_array();
    }
}

#[cfg(any(target_arch = "x86", target_arch = "x86_64"))]
#[cfg(all(target_feature = "ssse3", target_feature = "sse2"))]
impl Axpy<__m128i, __m128i> for __m128i {
    fn axpy(&mut self, a: &__m128i, x: &__m128i) {
        unsafe {
            let p = _mm_maddubs_epi16(*x, *a);
            let q = _mm_madd_epi16(p, _mm_set1_epi16(1));
            *self = _mm_add_epi32(*self, q);
        }
    }
}

impl Axpy<i8x16, i8x16> for i32x4 {
    fn axpy(&mut self, a: &i8x16, x: &i8x16) {
        let a = a.cast::<i32>();
        let a0 = simd_swizzle!(a, [0, 4, 8, 12]);
        let a1 = simd_swizzle!(a, [1, 5, 9, 13]);
        let a2 = simd_swizzle!(a, [2, 6, 10, 14]);
        let a3 = simd_swizzle!(a, [3, 7, 11, 15]);

        let x = x.cast::<i32>();
        let x0 = simd_swizzle!(x, [0, 4, 8, 12]);
        let x1 = simd_swizzle!(x, [1, 5, 9, 13]);
        let x2 = simd_swizzle!(x, [2, 6, 10, 14]);
        let x3 = simd_swizzle!(x, [3, 7, 11, 15]);

        *self += a0 * x0 + a1 * x1 + a2 * x2 + a3 * x3;
    }
}

impl<const I: usize, const O: usize> Axpy<[[i8; I]; O], [i8; I]> for [i32; O] {
    fn axpy(&mut self, a: &[[i8; I]; O], x: &[i8; I]) {
        for (o, y) in self.iter_mut().enumerate() {
            y.axpy(&a[o], x);
        }
    }
}

impl<T, const I: usize, const O: usize> Axpy<[[T; O]; I], [u16]> for [T; O]
where
    T: Copy + AddAssign,
{
    fn axpy(&mut self, a: &[[T; O]; I], x: &[u16]) {
        for i in x {
            self.axpy(a, i);
        }
    }
}

impl<T, const I: usize, const O: usize> Axpy<[[T; O]; I], u16> for [T; O]
where
    T: Copy + AddAssign,
{
    fn axpy(&mut self, a: &[[T; O]; I], &x: &u16) {
        let a = a.get(x as usize).assume();
        for (y, a) in self.iter_mut().zip(a) {
            *y += *a
        }
    }
}

#[derive(Debug, Copy, Clone, Eq, PartialEq, Ord, PartialOrd, Hash)]
pub struct Invert<T>(pub T);

impl<T, const I: usize, const O: usize> Axpy<[[T; O]; I], Invert<u16>> for [T; O]
where
    T: Copy + SubAssign,
{
    fn axpy(&mut self, a: &[[T; O]; I], &Invert(x): &Invert<u16>) {
        let a = a.get(x as usize).assume();
        for (y, a) in self.iter_mut().zip(a) {
            *y -= *a
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use test_strategy::proptest;

    #[cfg(any(target_arch = "x86", target_arch = "x86_64"))]
    #[cfg(target_feature = "avx2")]
    #[proptest]
    fn axpy_supports_avx2(a: [i8; 32], x: [i8; 32], y: [i32; 8]) {
        let x = x.map(|v| v.max(0));
        let (ap, xp, mut yp) = (i8x32::from(a), i8x32::from(x), i32x8::from(y));
        let (aq, xq, mut yq) = (__m256i::from(ap), __m256i::from(xp), __m256i::from(yp));

        yp.axpy(&ap, &xp);
        yq.axpy(&aq, &xq);

        assert_eq!(yp, yq.into());
    }

    #[cfg(any(target_arch = "x86", target_arch = "x86_64"))]
    #[cfg(all(target_feature = "ssse3", target_feature = "sse2"))]
    #[proptest]
    fn axpy_supports_ssse3(a: [i8; 16], x: [i8; 16], y: [i32; 4]) {
        let x = x.map(|v| v.max(0));
        let (ap, xp, mut yp) = (i8x16::from(a), i8x16::from(x), i32x4::from(y));
        let (aq, xq, mut yq) = (__m128i::from(ap), __m128i::from(xp), __m128i::from(yp));

        yp.axpy(&ap, &xp);
        yq.axpy(&aq, &xq);

        assert_eq!(yp, yq.into());
    }

    #[proptest]
    fn axpy_computes_inner_product_of_vectors(a: [i8; 50], x: [i8; 50]) {
        let x = x.map(|v| v.max(0));

        let mut y = 0;
        y.axpy(&a, &x);

        assert_eq!(y, a.iter().zip(x).map(|(&a, x)| a as i32 * x as i32).sum());
    }

    #[proptest]
    fn axpy_computes_inner_product_of_matrix_and_vector(a: [[i8; 50]; 10], x: [i8; 50]) {
        let x = x.map(|v| v.max(0));

        let mut y = [0; 10];
        y.axpy(&a, &x);

        assert_eq!(
            y,
            a.map(|a| a.iter().zip(x).map(|(&a, x)| a as i32 * x as i32).sum())
        );
    }

    #[proptest]
    fn axpy_swizzles_matrix(
        #[strategy([[-10..10i8, -10..10i8], [-10..10i8, -10..10i8], [-10..10i8, -10..10i8]])]
        a: [[i8; 2]; 3],
        #[strategy([0..3u16, 0..3u16, 0..3u16])] x: [u16; 3],
    ) {
        let mut y = [0; 2];
        y.axpy(&a, x.as_slice());

        assert_eq!(
            y,
            [
                a[x[0] as usize][0] + a[x[1] as usize][0] + a[x[2] as usize][0],
                a[x[0] as usize][1] + a[x[1] as usize][1] + a[x[2] as usize][1],
            ]
        );
    }
}
