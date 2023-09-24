use arrayvec::ArrayVec;
use derive_more::{DebugCustom, Deref, DerefMut};
use std::fmt::Debug;

#[cfg(test)]
use proptest::{collection::vec, prelude::*};

/// A buffer of fixed capacity.
#[derive(DebugCustom, Clone, Eq, PartialEq, Hash, Deref, DerefMut)]
#[cfg_attr(test, derive(test_strategy::Arbitrary))]
#[cfg_attr(test, arbitrary(bound(T: 'static + Debug + Arbitrary)))]
#[debug(bound = "T: Debug")]
#[debug(fmt = "Buffer({_0:?})")]
pub struct Buffer<T, const N: usize>(
    #[cfg_attr(test, strategy(vec(any::<T>(), 0..=N).prop_map(ArrayVec::from_iter)))]
    #[deref(forward)]
    #[deref_mut(forward)]
    ArrayVec<T, N>,
);

impl<T, const N: usize> Buffer<T, N> {
    /// Whether the buffer's len is equal to `N`.
    pub fn is_full(&self) -> bool {
        self.0.is_full()
    }

    /// Pushes an element in the back of the buffer.
    ///
    /// # Panics
    ///
    /// Panics if the buffer is full.
    pub fn push(&mut self, e: T) {
        self.0.push(e);
    }

    /// Pushes an element in front of the buffer.
    ///
    /// The buffer is truncated if it's full.
    ///
    /// # Panics
    ///
    /// Panics if `N` is `0`.
    pub fn shift(&mut self, e: T) {
        if self.is_full() {
            self.rotate_right(1);
            self[0] = e;
        } else {
            self.push(e);
            self.rotate_right(1);
        }
    }
}

/// Constructs an empty [`Buffer`].
impl<T, const N: usize> Default for Buffer<T, N> {
    fn default() -> Self {
        Self(Default::default())
    }
}

impl<T, const N: usize> IntoIterator for Buffer<T, N> {
    type Item = <ArrayVec<T, N> as IntoIterator>::Item;
    type IntoIter = <ArrayVec<T, N> as IntoIterator>::IntoIter;
    fn into_iter(self) -> Self::IntoIter {
        self.0.into_iter()
    }
}

impl<'a, T, const N: usize> IntoIterator for &'a Buffer<T, N> {
    type Item = <&'a ArrayVec<T, N> as IntoIterator>::Item;
    type IntoIter = <&'a ArrayVec<T, N> as IntoIterator>::IntoIter;
    fn into_iter(self) -> Self::IntoIter {
        (&self.0).into_iter()
    }
}

impl<'a, T, const N: usize> IntoIterator for &'a mut Buffer<T, N> {
    type Item = <&'a mut ArrayVec<T, N> as IntoIterator>::Item;
    type IntoIter = <&'a mut ArrayVec<T, N> as IntoIterator>::IntoIter;
    fn into_iter(self) -> Self::IntoIter {
        (&mut self.0).into_iter()
    }
}

/// Extends a [`Buffer`] with an iterator of elements.
///
/// The buffer might be truncated if the number of elements exceeds the internal capacity.
impl<T, const N: usize> Extend<T> for Buffer<T, N> {
    fn extend<I: IntoIterator<Item = T>>(&mut self, i: I) {
        let limit = N - self.len();
        self.0.extend(i.into_iter().take(limit));
    }
}

/// Create a [`Buffer`] from an iterator of elements.
///
/// The buffer might be truncated if the number of elements exceeds the internal capacity.
impl<T, const N: usize> FromIterator<T> for Buffer<T, N> {
    fn from_iter<I: IntoIterator<Item = T>>(i: I) -> Self {
        let mut ring = Buffer::default();
        ring.extend(i);
        ring
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use proptest::sample::size_range;
    use test_strategy::proptest;

    #[proptest]
    fn len_returns_number_of_elements_in_the_buffer(b: Buffer<u8, 3>) {
        assert_eq!(b.len(), b.iter().len());
    }

    #[proptest]
    fn is_empty_returns_whether_there_are_no_elements_in_the_buffer(b: Buffer<u8, 3>) {
        assert_eq!(b.is_empty(), b.iter().count() == 0);
    }

    #[proptest]
    fn is_full_returns_whether_there_are_no_capacity_left_in_the_buffer(b: Buffer<u8, 3>) {
        assert_eq!(b.is_full(), b.iter().count() == 3);
    }

    #[proptest]
    #[should_panic]
    fn push_panics_if_full(#[filter(#b.is_full())] mut b: Buffer<u8, 3>, e: u8) {
        b.push(e);
    }

    #[proptest]
    fn push_inserts_element_at_the_end(#[filter(!#b.is_full())] b: Buffer<u8, 3>, e: u8) {
        let mut c = b.clone();
        c.push(e);
        assert_eq!(c[b.len()], e);
        assert_eq!(c[..b.len()], b[..]);
    }

    #[proptest]
    #[should_panic]
    fn shift_panics_if_capacity_is_zero(mut b: Buffer<u8, 0>, e: u8) {
        b.shift(e);
    }

    #[proptest]
    fn shift_does_not_truncate_if_not_full(#[filter(!#b.is_full())] b: Buffer<u8, 3>, e: u8) {
        let mut c = b.clone();
        c.shift(e);
        assert_eq!(c[0], e);
        assert_eq!(c[1..], b[..]);
    }

    #[proptest]
    fn shift_truncates_if_full(#[filter(#b.is_full())] b: Buffer<u8, 3>, e: u8) {
        let mut c = b.clone();
        c.shift(e);
        assert_eq!(c[0], e);
        assert_eq!(c[1..], b[..2]);
    }

    #[proptest]
    fn from_iterator_truncates(#[any(size_range(0..=6).lift())] v: Vec<u8>) {
        assert_eq!(
            Buffer::<_, 3>::from_iter(v.clone()),
            v.into_iter().take(3).collect()
        );
    }

    #[proptest]
    fn extend_truncates(mut b: Buffer<u8, 3>, #[any(size_range(0..=6).lift())] v: Vec<u8>) {
        let l = b.len();
        b.extend(v.clone());
        assert_eq!(b[l..], v[..v.len().min(3usize.saturating_sub(l))]);
    }
}
