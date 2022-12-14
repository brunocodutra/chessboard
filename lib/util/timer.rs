use derive_more::{Display, Error};
use std::time::{Duration, Instant};
use test_strategy::Arbitrary;

#[derive(Debug, Display, Copy, Clone, Eq, PartialEq, Ord, PartialOrd, Error)]
#[display(fmt = "time is up!")]
pub struct Timeout;

#[derive(Debug, Default, Copy, Clone, Eq, PartialEq, Hash, Arbitrary)]
pub struct Timer {
    deadline: Option<Instant>,
}

impl Timer {
    pub fn start(duration: Duration) -> Self {
        Timer {
            deadline: Instant::now().checked_add(duration),
        }
    }

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
