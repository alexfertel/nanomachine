use core::{
    fmt::{Debug, Display, Formatter},
    hash::Hash,
};

/// Errors that can occur when triggering events on a [`Machine`].
///
/// This error type is returned by [`Machine::trigger`] and
/// [`Machine::trigger_with`] to indicate invalid operations.
#[derive(Debug, Clone, Eq, Hash, PartialEq)]
pub enum MachineError {
    /// The specified event is not defined in the state machine.
    EventInvalid,
    /// The specified event is defined for this machine, but not valid from the
    /// current state.
    StateInvalid,
}

impl Display for MachineError {
    fn fmt(&self, f: &mut Formatter<'_>) -> core::fmt::Result {
        match self {
            MachineError::EventInvalid => write!(
                f,
                "The specified event is not defined in this state machine"
            ),
            MachineError::StateInvalid => {
                write!(f, "The event is not valid for the current state")
            }
        }
    }
}

impl core::error::Error for MachineError {}
