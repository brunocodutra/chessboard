use crate::util::Assume;
use derive_more::{Deref, DerefMut, From};
use std::ops::AddAssign;

#[cfg(test)]
use proptest::prelude::*;

#[cfg(test)]
use std::fmt::Debug;

/// A 1D array.
#[derive(Debug, Copy, Clone, Eq, PartialEq, Hash, From, Deref, DerefMut)]
#[cfg_attr(test, derive(test_strategy::Arbitrary))]
#[cfg_attr(test, arbitrary(bound(T: 'static + Debug + Arbitrary)))]
#[repr(C, align(64))]
pub struct Vector<T, const N: usize>(pub(crate) [T; N]);

impl<T: Default + Copy, const N: usize> Default for Vector<T, N> {
    fn default() -> Self {
        Vector([T::default(); N])
    }
}

impl<T, const N: usize> Vector<T, N> {
    pub fn map<U>(self, f: fn(T) -> U) -> Vector<U, N> {
        Vector(self.0.map(f))
    }
}

/// A 2D array.
#[derive(Debug, Copy, Clone, Eq, PartialEq, Hash, From, Deref, DerefMut)]
#[cfg_attr(test, derive(test_strategy::Arbitrary))]
#[cfg_attr(test, arbitrary(bound(T: 'static + Debug + Arbitrary)))]
#[repr(C, align(64))]
pub struct Matrix<T, const I: usize, const O: usize>(pub(crate) [[T; I]; O]);

impl<T: Default + Copy, const I: usize, const O: usize> Default for Matrix<T, I, O> {
    fn default() -> Self {
        Matrix([[T::default(); I]; O])
    }
}

/// A trait for types that implement affine transformations.
pub trait Axpy<A: ?Sized, X: ?Sized> {
    /// Computes `self += a * x`.
    fn axpy(&mut self, a: &A, x: &X);
}

impl<const I: usize> Axpy<Vector<i8, I>, Vector<i8, I>> for i32 {
    fn axpy(&mut self, a: &Vector<i8, I>, x: &Vector<i8, I>) {
        for i in 0..I {
            *self += a[i] as i32 * x[i] as i32;
        }
    }
}

impl<const I: usize, const O: usize> Axpy<Matrix<i8, I, O>, Vector<i8, I>> for Vector<i32, O> {
    fn axpy(&mut self, a: &Matrix<i8, I, O>, x: &Vector<i8, I>) {
        let mut chunks = self.chunks_exact_mut(8);

        for (o, y) in (&mut chunks).enumerate() {
            for i in 0..I {
                y[0] += x[i] as i32 * a[o * 8][i] as i32;
                y[1] += x[i] as i32 * a[o * 8 + 1][i] as i32;
                y[2] += x[i] as i32 * a[o * 8 + 2][i] as i32;
                y[3] += x[i] as i32 * a[o * 8 + 3][i] as i32;
                y[4] += x[i] as i32 * a[o * 8 + 4][i] as i32;
                y[5] += x[i] as i32 * a[o * 8 + 5][i] as i32;
                y[6] += x[i] as i32 * a[o * 8 + 6][i] as i32;
                y[7] += x[i] as i32 * a[o * 8 + 7][i] as i32;
            }
        }

        for (o, y) in chunks.into_remainder().iter_mut().enumerate() {
            for i in 0..I {
                *y += x[i] as i32 * a[o + (O / 8) * 8][i] as i32;
            }
        }
    }
}

impl<T, const I: usize, const O: usize> Axpy<Matrix<T, O, I>, [u16]> for Vector<T, O>
where
    T: Copy + AddAssign,
{
    fn axpy(&mut self, a: &Matrix<T, O, I>, x: &[u16]) {
        let mut chunks = x.chunks_exact(8);

        for i in &mut chunks {
            for (o, y) in self.iter_mut().enumerate() {
                *y += a.get(i[0] as usize).assume()[o];
                *y += a.get(i[1] as usize).assume()[o];
                *y += a.get(i[2] as usize).assume()[o];
                *y += a.get(i[3] as usize).assume()[o];
                *y += a.get(i[4] as usize).assume()[o];
                *y += a.get(i[5] as usize).assume()[o];
                *y += a.get(i[6] as usize).assume()[o];
                *y += a.get(i[7] as usize).assume()[o];
            }
        }

        let mut chunks = chunks.remainder().chunks_exact(4);

        for i in &mut chunks {
            for (o, y) in self.iter_mut().enumerate() {
                *y += a.get(i[0] as usize).assume()[o];
                *y += a.get(i[1] as usize).assume()[o];
                *y += a.get(i[2] as usize).assume()[o];
                *y += a.get(i[3] as usize).assume()[o];
            }
        }

        for i in chunks.remainder() {
            for (o, y) in self.iter_mut().enumerate() {
                *y += a.get(*i as usize).assume()[o]
            }
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use test_strategy::proptest;

    #[proptest]
    fn axpy_computes_inner_product_of_vectors(a: Vector<i8, 2>, x: Vector<i8, 2>) {
        let mut y = 0;
        y.axpy(&a, &x);
        assert_eq!(y, a[0] as i32 * x[0] as i32 + a[1] as i32 * x[1] as i32);
    }

    #[proptest]
    fn axpy_computes_inner_product_of_matrix_and_vector(a: Matrix<i8, 2, 10>, x: Vector<i8, 2>) {
        let mut y = Vector::default();
        y.axpy(&a, &x);

        assert_eq!(
            y,
            Vector([
                a[0][0] as i32 * x[0] as i32 + a[0][1] as i32 * x[1] as i32,
                a[1][0] as i32 * x[0] as i32 + a[1][1] as i32 * x[1] as i32,
                a[2][0] as i32 * x[0] as i32 + a[2][1] as i32 * x[1] as i32,
                a[3][0] as i32 * x[0] as i32 + a[3][1] as i32 * x[1] as i32,
                a[4][0] as i32 * x[0] as i32 + a[4][1] as i32 * x[1] as i32,
                a[5][0] as i32 * x[0] as i32 + a[5][1] as i32 * x[1] as i32,
                a[6][0] as i32 * x[0] as i32 + a[6][1] as i32 * x[1] as i32,
                a[7][0] as i32 * x[0] as i32 + a[7][1] as i32 * x[1] as i32,
                a[8][0] as i32 * x[0] as i32 + a[8][1] as i32 * x[1] as i32,
                a[9][0] as i32 * x[0] as i32 + a[9][1] as i32 * x[1] as i32,
            ])
        );
    }

    #[proptest]
    fn axpy_swizzles_matrix(
        #[strategy([
            [-10..10i8, -10..10i8], [-10..10i8, -10..10i8],
            [-10..10i8, -10..10i8], [-10..10i8, -10..10i8],
            [-10..10i8, -10..10i8], [-10..10i8, -10..10i8],
            [-10..10i8, -10..10i8], [-10..10i8, -10..10i8],
            [-10..10i8, -10..10i8], [-10..10i8, -10..10i8],
        ])]
        a: [[i8; 2]; 10],
        #[strategy([
            0..10u16, 0..10u16, 0..10u16, 0..10u16, 0..10u16,
            0..10u16, 0..10u16, 0..10u16, 0..10u16, 0..10u16,
        ])]
        x: [u16; 10],
    ) {
        let mut y = Vector::<_, 2>::default();
        y.axpy(&Matrix(a), &x);

        assert_eq!(
            y,
            Vector([
                a[x[0] as usize][0]
                    + a[x[1] as usize][0]
                    + a[x[2] as usize][0]
                    + a[x[3] as usize][0]
                    + a[x[4] as usize][0]
                    + a[x[5] as usize][0]
                    + a[x[6] as usize][0]
                    + a[x[7] as usize][0]
                    + a[x[8] as usize][0]
                    + a[x[9] as usize][0],
                a[x[0] as usize][1]
                    + a[x[1] as usize][1]
                    + a[x[2] as usize][1]
                    + a[x[3] as usize][1]
                    + a[x[4] as usize][1]
                    + a[x[5] as usize][1]
                    + a[x[6] as usize][1]
                    + a[x[7] as usize][1]
                    + a[x[8] as usize][1]
                    + a[x[9] as usize][1],
            ])
        );
    }
}
