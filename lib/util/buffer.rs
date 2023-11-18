use arrayvec::ArrayVec;
use derive_more::{Debug, Deref, DerefMut, IntoIterator};
use std::mem::swap;

#[cfg(test)]
use proptest::{collection::vec, prelude::*};

/// A stack allocated buffer of fixed capacity.
#[derive(Debug, Clone, Eq, PartialEq, Hash, Deref, DerefMut, IntoIterator)]
#[cfg_attr(test, derive(test_strategy::Arbitrary))]
#[cfg_attr(test, arbitrary(bound(T: 'static + Arbitrary)))]
#[debug("Buffer({_0:?})")]
pub struct Buffer<T, const N: usize>(
    #[cfg_attr(test, strategy(vec(any::<T>(), 0..=N).prop_map(ArrayVec::from_iter)))]
    #[deref(forward)]
    #[deref_mut(forward)]
    ArrayVec<T, N>,
);

impl<T, const N: usize> Buffer<T, N> {
    /// Constructs an empty buffer.
    pub fn new() -> Self {
        Default::default()
    }

    /// Whether the buffer's len is equal to `N`.
    pub fn is_full(&self) -> bool {
        self.0.is_full()
    }

    /// Attempts to pop an element from the back of the buffer.
    ///
    /// Returns `None` if the buffer is empty.
    pub fn pop(&mut self) -> Option<T> {
        self.0.pop()
    }

    /// Attempts to push an element in the back of the buffer.
    ///
    /// Returns `Some(e)` if the buffer is full.
    pub fn push(&mut self, e: T) -> Option<T> {
        match self.0.try_push(e) {
            Err(e) => Some(e.element()),
            Ok(_) => None,
        }
    }

    /// Pushes an element in front of the buffer.
    ///
    /// Returns the previous last element if the buffer is full.
    ///
    /// # Panics
    ///
    /// Panics if `N` is `0`.
    pub fn shift(&mut self, mut e: T) -> Option<T> {
        if self.is_full() {
            self.rotate_right(1);
            swap(&mut self[0], &mut e);
            Some(e)
        } else {
            self.push(e);
            self.rotate_right(1);
            None
        }
    }
}

/// Constructs an empty [`Buffer`].
impl<T, const N: usize> Default for Buffer<T, N> {
    fn default() -> Self {
        Self(Default::default())
    }
}

/// Create a [`Buffer`] from an iterator of elements.
///
/// # Panics
///
/// If the number of elements in the iterator exceeds the buffer's capacity.
impl<T, const N: usize> FromIterator<T> for Buffer<T, N> {
    fn from_iter<I: IntoIterator<Item = T>>(i: I) -> Self {
        Buffer(ArrayVec::from_iter(i))
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use proptest::sample::size_range;
    use std::fmt::Debug;
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
    fn pop_returns_none_if_empty() {
        assert_eq!(Buffer::<u8, 3>::new().pop(), None);
    }

    #[proptest]
    fn pop_removes_element_from_the_end(#[filter(!#b.is_empty())] b: Buffer<u8, 3>) {
        let mut c = b.clone();
        assert_eq!(c.pop(), Some(b[c.len()]));
        assert_eq!(b[..c.len()], c[..]);
    }

    #[proptest]
    fn push_returns_some_if_full(#[filter(#b.is_full())] mut b: Buffer<u8, 3>, e: u8) {
        assert_eq!(b.push(e), Some(e));
    }

    #[proptest]
    fn push_inserts_element_at_the_end(#[filter(!#b.is_full())] b: Buffer<u8, 3>, e: u8) {
        let mut c = b.clone();
        assert_eq!(c.push(e), None);
        assert_eq!(c[b.len()], e);
        assert_eq!(c[..b.len()], b[..]);
    }

    #[proptest]
    #[should_panic]
    fn shift_panics_if_capacity_is_zero(mut b: Buffer<u8, 0>, e: u8) {
        b.shift(e);
    }

    #[proptest]
    fn shift_inserts_element_at_the_front(#[filter(!#b.is_full())] b: Buffer<u8, 3>, e: u8) {
        let mut c = b.clone();
        assert_eq!(c.shift(e), None);
        assert_eq!(c[0], e);
        assert_eq!(c[1..], b[..]);
    }

    #[proptest]
    fn shift_returns_previous_last_element_if_full(
        #[filter(#b.is_full())] b: Buffer<u8, 3>,
        e: u8,
    ) {
        let mut c = b.clone();
        assert_eq!(c.shift(e), Some(b[b.len() - 1]));
        assert_eq!(c[0], e);
        assert_eq!(c[1..], b[..2]);
    }

    #[proptest]
    #[should_panic]
    fn from_iterator_panics_on_overflow(#[any(size_range(4..=10).lift())] v: Vec<u8>) {
        Buffer::<_, 3>::from_iter(v);
    }
}
