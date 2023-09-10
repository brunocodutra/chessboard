use derive_more::{Display, Error};
use std::time::{Duration, Instant};

#[derive(Debug, Display, Copy, Clone, Eq, PartialEq, Ord, PartialOrd, Error)]
#[display(fmt = "time is up!")]
pub struct Timeout;

#[derive(Debug, Copy, Clone, Eq, PartialEq, Hash)]
#[cfg_attr(test, derive(test_strategy::Arbitrary))]
pub struct Timer {
    deadline: Option<Instant>,
}

impl Timer {
    /// Constructs a timer that never elapses.
    pub fn disarmed() -> Self {
        Timer { deadline: None }
    }

    /// Constructs a timer that elapses after the given duration.
    pub fn start(duration: Duration) -> Self {
        Timer {
            deadline: Instant::now().checked_add(duration),
        }
    }

    /// Checks whether the timer has elapsed.
    pub fn elapsed(&self) -> Result<(), Timeout> {
        if self.deadline.map(|t| t.elapsed()) > Some(Duration::ZERO) {
            Err(Timeout)
        } else {
            Ok(())
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::thread::sleep;
    use test_strategy::proptest;

    #[proptest]
    fn timer_does_not_elapse_before_duration_expires() {
        let timer = Timer::start(Duration::MAX);
        assert_eq!(timer.elapsed(), Ok(()))
    }

    #[proptest]
    fn timer_elapses_once_duration_expires() {
        let timer = Timer::start(Duration::ZERO);
        sleep(Duration::from_millis(1));
        assert_eq!(timer.elapsed(), Err(Timeout))
    }
}
