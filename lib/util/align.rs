use derive_more::{Deref, DerefMut, IntoIterator};

#[cfg(test)]
use proptest::prelude::*;

#[derive(Debug, Default, Copy, Clone, Eq, PartialEq, Hash, Deref, DerefMut, IntoIterator)]
#[cfg_attr(test, derive(test_strategy::Arbitrary))]
#[cfg_attr(test, arbitrary(bound(T: 'static + Arbitrary)))]
#[repr(align(64))]
pub struct AlignTo64<T>(#[cfg_attr(test, into_iterator(owned, ref, ref_mut))] pub T);
