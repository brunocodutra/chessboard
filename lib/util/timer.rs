use std::time::{Duration, Instant};

/// Tracks time towards a deadline.
#[derive(Debug, Default)]
pub struct Timer {
    deadline: Option<Instant>,
}

impl Timer {
    /// Constructs a timer that elapses after the given duration.
    #[inline(always)]
    pub const fn infinite() -> Self {
        Timer { deadline: None }
    }

    /// Constructs a timer that elapses after the given duration.
    #[inline(always)]
    pub fn new(duration: Duration) -> Self {
        Timer {
            deadline: Instant::now().checked_add(duration),
        }
    }

    /// Returns the time remaining if any.
    #[inline(always)]
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

    #[test]
    fn timer_measures_time_remaining() {
        let timer = Timer::new(Duration::from_secs(1));
        assert!(timer.remaining().is_some());
    }

    #[test]
    fn timer_elapses_once_duration_expires() {
        let timer = Timer::new(Duration::ZERO);
        sleep(Duration::from_millis(1));
        assert_eq!(timer.remaining(), None);
    }

    #[test]
    fn timer_never_decreases_if_infinite() {
        let timer = Timer::infinite();
        let remaining = timer.remaining();
        sleep(Duration::from_millis(1));
        assert_eq!(remaining, timer.remaining());
    }
}
