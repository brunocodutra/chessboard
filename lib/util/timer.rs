use std::time::{Duration, Instant};

/// Tracks time towards a deadline.
#[derive(Debug)]
#[cfg_attr(test, derive(test_strategy::Arbitrary))]
pub struct Timer {
    deadline: Option<Instant>,
}

impl Timer {
    /// Constructs a timer that elapses after the given duration.
    pub fn new(duration: Duration) -> Self {
        Timer {
            deadline: Instant::now().checked_add(duration),
        }
    }

    /// Returns the time remaining if any.
    pub fn remaining(&self) -> Option<Duration> {
        match self.deadline {
            Some(deadline) => deadline.checked_duration_since(Instant::now()),
            None => Some(Duration::MAX),
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::thread::sleep;
    use test_strategy::proptest;

    #[proptest]
    fn timer_measures_time_remaining() {
        let timer = Timer::new(Duration::from_secs(1));
        assert!(timer.remaining().is_some());
    }

    #[proptest]
    fn timer_does_not_elapse_before_duration_expires() {
        let timer = Timer::new(Duration::MAX);
        assert_eq!(timer.remaining(), Some(Duration::MAX));
    }

    #[proptest]
    fn timer_elapses_once_duration_expires() {
        let timer = Timer::new(Duration::ZERO);
        sleep(Duration::from_millis(1));
        assert_eq!(timer.remaining(), None);
    }
}
