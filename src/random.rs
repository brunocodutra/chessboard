use derive_more::DebugCustom;
use rand::distributions::{Distribution, Standard};
use rand::{rngs::StdRng, Rng, RngCore, SeedableRng};
use std::sync::{Arc, Mutex};
use tracing::instrument;

/// A dynamically dispatched, thread-safe wrapper for any type that implements [`rand::RngCore`].
#[derive(DebugCustom, Clone)]
#[debug(fmt = "Random")]
pub struct Random(Arc<Mutex<dyn RngCore + Send + 'static>>);

impl Random {
    /// Constructs [`Random`] from any type that implements [`rand::RngCore`].
    pub fn new<R: RngCore + Send + 'static>(rng: R) -> Self {
        Random(Arc::new(Mutex::new(rng)))
    }

    /// Samples a random number from the standard distribution.
    ///
    /// See also [`rand::Rng::gen`].
    #[instrument(level = "trace")]
    pub fn gen<T>(&self) -> T
    where
        Standard: Distribution<T>,
    {
        self.0.lock().unwrap().gen()
    }
}

/// Initializes a [`rand::rngs::StdRng`] seeded by [system entropy].
///
/// [system entropy]: rand::rngs::StdRng::from_entropy
impl Default for Random {
    fn default() -> Self {
        Random::new(StdRng::from_entropy())
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use proptest::prelude::*;
    use rand::rngs::mock::StepRng;

    proptest! {
        #[test]
        fn gen_returns_random_number(n: u64) {
            assert_eq!(Random::new(StepRng::new(n, 0)).gen::<u64>(), n);
        }
    }
}
