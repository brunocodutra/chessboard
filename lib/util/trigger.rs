use std::sync::atomic::{AtomicBool, Ordering};

/// A one-shot trigger.
#[derive(Debug)]
pub struct Trigger(AtomicBool);

impl Trigger {
    /// An armed trigger.
    #[inline(always)]
    pub const fn armed() -> Self {
        Trigger(AtomicBool::new(true))
    }

    /// A disarmed trigger.
    #[inline(always)]
    pub const fn disarmed() -> Self {
        Trigger(AtomicBool::new(false))
    }

    /// Disarm the trigger.
    ///
    /// Returns `true` if the trigger was disarmed for the first time.
    #[inline(always)]
    pub fn disarm(&self) -> bool {
        self.0.fetch_and(false, Ordering::Relaxed)
    }

    /// Whether the trigger is armed.
    #[inline(always)]
    pub fn is_armed(&self) -> bool {
        self.0.load(Ordering::Relaxed)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn trigger_can_only_be_disarmed_once() {
        let trigger = Trigger::armed();
        assert!(trigger.is_armed());
        assert!(trigger.disarm());
        assert!(!trigger.is_armed());
        assert!(!trigger.disarm());
    }
}
