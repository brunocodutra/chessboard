use crate::util::Saturating;

#[cfg(not(test))]
pub type Draft = Saturating<i8, -31, 31>;

#[cfg(test)]
pub type Draft = Saturating<i8, -3, 3>;
