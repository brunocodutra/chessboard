use crate::util::{Counter, Timer, Trigger};
use derive_more::{Display, Error};

/// Indicates the search was interrupted .
#[derive(Debug, Display, Copy, Clone, Eq, PartialEq, Ord, PartialOrd, Hash, Error)]
#[display("the search was interrupted")]
pub struct Interrupted;

/// The search control.
#[derive(Debug, Default)]
pub enum Control<'a> {
    #[default]
    Unlimited,
    Limited(Counter, Timer, &'a Trigger),
}

impl<'a> Control<'a> {
    /// A reference to the timer.
    #[inline(always)]
    pub fn timer(&self) -> &Timer {
        match self {
            Control::Unlimited => &const { Timer::infinite() },
            Control::Limited(_, timer, _) => timer,
        }
    }

    /// Whether the search should be interrupted.
    #[inline(always)]
    pub fn interrupted(&self) -> Result<(), Interrupted> {
        if let Control::Limited(nodes, timer, trigger) = self {
            nodes.count().ok_or(Interrupted)?;
            timer.remaining().ok_or(Interrupted)?;
            if !trigger.is_armed() {
                return Err(Interrupted);
            }
        }

        Ok(())
    }
}
