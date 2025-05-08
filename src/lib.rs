//! A generic finite state machine implementation.
//!
//! This module provides a `Machine` struct that represents a finite state
//! machine (FSM) with generic state (`S`) and event (`E`) types.
//!
//! You can define transitions between states based on events, register
//! callbacks for entering specific states or for any transition, and trigger
//! events with optional payloads.
//!
//! # Examples
//!
//! ```rust
//! use nanomachine::Machine;
//!
//! #[derive(Debug, Clone, PartialEq, Eq, Hash)]
//! enum State {
//!   Locked,
//!   Unlocked,
//! }
//!
//! #[derive(Debug, Clone, PartialEq, Eq, Hash)]
//! enum Event {
//!     InsertCoin,
//!     TurnKnob,
//! }
//!
//! let mut nano = Machine::new(State::Locked);
//!
//! // Define transitions.
//! nano.when(Event::InsertCoin, State::Locked, State::Unlocked);
//! nano.when(Event::TurnKnob, State::Unlocked, State::Locked);
//!
//! // Register a callback when entering Unlocked.
//! nano.on_enter(State::Unlocked, |event| {
//!     println!("Unlocked by event: {:?}", event);
//! });
//!
//! assert!(nano.trigger(&Event::InsertCoin).is_ok());
//! assert_eq!(*nano.state(), State::Unlocked);
//!
//! assert!(nano.trigger(&Event::TurnKnob).is_ok());
//! assert_eq!(*nano.state(), State::Locked);
//!
//! // You can attach data to transitions.
//! nano.on_enter_with(State::Unlocked, |event, amount: &u32| {
//!     println!("Unlocked after {} cents by {:?}", amount, event);
//! });
//!
//! // Pass a payload when triggering.
//! nano.trigger_with(&Event::InsertCoin, &50u32);
//! ```

#![warn(clippy::perf, clippy::pedantic, missing_docs)]
#![no_std]

mod error;
pub use error::MachineError;

extern crate alloc;

use alloc::{rc::Rc, vec::Vec};
use core::{any::Any, fmt::Debug, hash::Hash};

use hashbrown::{HashMap, HashSet};

/// A specialized `Result` type for operations on a [`Machine`].
///
/// This is an alias for `core::result::Result<T, MachineError>`.
pub type MachineResult<T> = core::result::Result<T, MachineError>;

/// A trigger key for callbacks, either targeting a specific state or any state.
#[derive(Debug, Clone, Eq, Hash, PartialEq)]
enum Trigger<S> {
    /// Callback should fire when entering this specific state.
    State(S),
    /// Callback should fire on any state transition.
    AnyState,
}

/// Any `Fn` that takes an event and some arbitrary payload as input.
type Callback<E> = Rc<dyn Fn(E, &dyn Any)>;

/// A generic finite state machine.
///
/// # Type Parameters
/// - `S`: The state type. Must implement `Eq + Hash + Clone`.
/// - `E`: The event type. Must implement `Eq + Hash + Clone`.
#[derive(Clone)]
pub struct Machine<S, E> {
    state: S,
    transitions: HashMap<E, HashMap<S, S>>,
    callbacks: HashMap<Trigger<S>, Vec<Callback<E>>>,
}

impl<S, E> Machine<S, E> {
    /// Create a new state machine with the given initial state.
    pub fn new(initial_state: S) -> Self {
        Machine {
            state: initial_state,
            transitions: HashMap::new(),
            callbacks: HashMap::new(),
        }
    }

    /// A reference to the current state of the machine.
    #[inline]
    pub fn state(&self) -> &S {
        &self.state
    }
}

impl<S, E> Machine<S, E>
where
    S: Eq + Hash + Clone,
    E: Eq + Hash + Clone,
{
    /// Returns an iterator over all states known to the machine.
    pub fn states(&self) -> impl Iterator<Item = &S> {
        let mut used = HashSet::new();
        for state_map in self.transitions.values() {
            for (from, to) in state_map {
                used.insert(from);
                used.insert(to);
            }
        }
        used.into_iter()
    }

    /// Returns an iterator over all events the machine can react to.
    #[inline]
    pub fn events(&self) -> impl Iterator<Item = &E> {
        self.transitions.keys()
    }

    /// Returns an iterator over events valid from the current state.
    ///
    /// Only events that have a defined transition from the machine's current
    /// state are included.
    pub fn triggerable_events(&self) -> impl Iterator<Item = &E> {
        self.transitions
            .iter()
            .filter(|(_, mp)| mp.get(self.state()).is_some())
            .map(|(e, _)| e)
    }
}

impl<S, E> Machine<S, E>
where
    S: Eq + Hash + Clone,
    E: Eq + Hash + Clone,
{
    /// When `event` occurs in `state`, move to `new_state`.
    ///
    /// Multiple calls to `when` for the same `(event, state)` will overwrite
    /// the previous `new_state`.
    pub fn when(&mut self, event: E, state: S, new_state: S) {
        self.transitions.entry(event).or_default().insert(state, new_state);
    }

    /// Define multiple transitions for a single event.
    ///
    /// The `mapping` iterator should yield `(from_state, to_state)` pairs.
    pub fn when_iter<I>(&mut self, event: E, mapping: I)
    where
        I: IntoIterator<Item = (S, S)>,
    {
        self.transitions.entry(event).or_default().extend(mapping);
    }

    /// Internal helper to wrap a callback that expects a specific payload type
    /// `P`.
    #[doc(hidden)]
    fn wrap_callback<P, F>(callback: F) -> Callback<E>
    where
        P: 'static,
        F: Fn(E, &P) + 'static,
    {
        Rc::new(move |evt, payload| {
            if let Some(p) = payload.downcast_ref::<P>() {
                callback(evt, p);
            }
        })
    }

    /// Register a callback that only cares about the event (no payload).
    pub fn on_enter<F>(&mut self, state: S, callback: F)
    where
        F: Fn(E) + 'static,
    {
        let callback: Callback<E> = Rc::new(move |evt, _payload| {
            callback(evt);
        });
        self.callbacks.entry(Trigger::State(state)).or_default().push(callback);
    }

    /// Register a callback that expects a payload of type P.
    ///
    /// The callback takes the triggering event and a payload of type `P`. It
    /// will only be invoked if the payload downcasts to `P` successfully.
    pub fn on_enter_with<P, F>(&mut self, state: S, callback: F)
    where
        P: 'static,
        F: Fn(E, &P) + 'static,
    {
        let callback = Self::wrap_callback(callback);
        self.callbacks.entry(Trigger::State(state)).or_default().push(callback);
    }

    /// Register a callback to fire on any state transition.
    ///
    /// Works similarly to `on_enter`, but the callback runs regardless of the
    /// specific state entered.
    pub fn on_transition<F>(&mut self, callback: F)
    where
        F: Fn(E) + 'static + Clone,
    {
        let callback: Callback<E> = Rc::new(move |evt, _payload| {
            callback(evt);
        });
        self.callbacks.entry(Trigger::AnyState).or_default().push(callback);
    }

    /// Register a callback to fire on any state transition with a payload of
    /// type `P`.
    ///
    /// Works similarly to `on_enter_with`, but the callback runs regardless of
    /// the specific state, and only if the payload downcasts to `P`
    /// successfully.
    pub fn on_transition_with<P, F>(&mut self, callback: F)
    where
        P: 'static,
        F: Fn(E, &P) + 'static + Clone,
    {
        let callback = Self::wrap_callback(callback);
        self.callbacks.entry(Trigger::AnyState).or_default().push(callback);
    }

    /// Trigger the given `event` on the machine without any payload.
    ///
    /// If the event is defined for the current state, the machine will
    /// transition to the corresponding new state and invoke any registered
    /// callbacks.
    ///
    /// # Errors
    ///
    /// - Returns [`MachineError::EventInvalid`] if the event is not defined in
    ///   this state machine.
    /// - Returns [`MachineError::StateInvalid`] if the event has no transition
    ///   defined for the machine's current state.
    #[inline]
    pub fn trigger(&mut self, event: &E) -> Result<(), MachineError> {
        self.trigger_with(event, &())
    }

    /// Trigger the given `event` on the machine with an associated payload.
    ///
    /// The payload will be provided to callbacks that accept the payload type
    /// `P`. If the event is defined for the current state, the machine will
    ///  perform the transition and invoke any matching callbacks.
    ///
    /// # Errors
    ///
    /// - Returns [`MachineError::EventInvalid`] if the event is not defined in
    ///   this state machine.
    /// - Returns [`MachineError::StateInvalid`] if no transition is defined for
    ///   the machine's current state with the given event.
    pub fn trigger_with<P>(
        &mut self,
        event: &E,
        payload: &P,
    ) -> Result<(), MachineError>
    where
        P: 'static,
    {
        let Some(state_map) = self.transitions.get(event) else {
            return Err(MachineError::EventInvalid);
        };

        let Some(new_state) = state_map.get(&self.state) else {
            return Err(MachineError::StateInvalid);
        };

        self.state = new_state.clone();
        let state_cbs = self.callbacks.get(&Trigger::State(self.state.clone()));
        let any_cbs = self.callbacks.get(&Trigger::AnyState);
        for cb in state_cbs.into_iter().chain(any_cbs.into_iter()).flatten() {
            cb(event.clone(), payload as &dyn Any);
        }

        Ok(())
    }
}

impl<S, E> Default for Machine<S, E>
where
    S: Default,
{
    /// Create a default machine, using `S::default()` as the initial state.
    fn default() -> Self {
        Machine::new(S::default())
    }
}

impl<S: Debug + Eq + Hash + Clone, E: Debug + Eq + Hash + Clone> Debug
    for Machine<S, E>
{
    fn fmt(&self, f: &mut core::fmt::Formatter<'_>) -> core::fmt::Result {
        f.debug_struct("Machine")
            .field("state", &self.state)
            .field("events", &self.transitions.keys().collect::<Vec<_>>())
            .field("callbacks", &self.callbacks.len())
            .finish()
    }
}

#[cfg(test)]
mod tests {
    use alloc::{
        rc::Rc,
        string::{String, ToString},
        vec::Vec,
    };
    use core::cell::Cell;

    use super::*;

    #[derive(Debug, Clone, PartialEq, Eq, Hash)]
    enum TestState {
        Idle,
        Running,
        Paused,
        Stopped,
    }

    #[derive(Debug, Clone, PartialEq, Eq, Hash)]
    enum TestEvent {
        Start,
        Pause,
        Resume,
        Stop,
    }

    fn create_machine() -> Machine<TestState, TestEvent> {
        let mut m = Machine::new(TestState::Idle);

        m.when_iter(
            TestEvent::Start,
            [
                (TestState::Idle, TestState::Running),
                (TestState::Stopped, TestState::Running),
            ],
        );

        m.when(TestEvent::Pause, TestState::Running, TestState::Paused);
        m.when(TestEvent::Resume, TestState::Paused, TestState::Running);
        m.when(TestEvent::Stop, TestState::Running, TestState::Stopped);

        m
    }

    #[test]
    fn initial_state() {
        let m = Machine::<usize, ()>::new(42);
        assert_eq!(*m.state(), 42);
    }

    #[test]
    fn valid_transition() {
        let mut m = create_machine();
        assert!(m.trigger(&TestEvent::Start).is_ok());
        assert_eq!(*m.state(), TestState::Running);
    }

    #[test]
    fn invalid_transition() {
        let mut m = create_machine();
        assert_eq!(
            m.trigger(&TestEvent::Pause).unwrap_err(),
            MachineError::StateInvalid
        );
        assert_eq!(*m.state(), TestState::Idle);
    }

    #[test]
    fn state_specific_callback() {
        let mut m = create_machine();
        let callback_called = Rc::new(Cell::new(false));

        let cc = callback_called.clone();
        m.on_enter_with(TestState::Running, move |_, _: &()| {
            cc.set(true);
        });

        m.trigger(&TestEvent::Start).unwrap();
        assert!(callback_called.get());
    }

    #[test]
    fn any_state_callback() {
        let mut m = create_machine();
        let callback_count = Rc::new(Cell::new(0));

        let cc = callback_count.clone();
        m.on_transition(move |_| {
            cc.set(cc.get() + 1);
        });

        m.trigger(&TestEvent::Start).unwrap();
        m.trigger(&TestEvent::Pause).unwrap();
        assert_eq!(callback_count.get(), 2);
    }

    #[test]
    fn payload_handling() {
        let mut m = Machine::new(TestState::Idle);
        let payload_received = Rc::new(Cell::new(None));

        m.when(TestEvent::Start, TestState::Idle, TestState::Running);

        let pr = payload_received.clone();
        m.on_enter_with(TestState::Running, move |_, p: &String| {
            pr.set(Some(p.clone()));
        });

        m.trigger_with(&TestEvent::Start, &"test payload".to_string()).unwrap();
        assert_eq!(payload_received.take(), Some("test payload".to_string()));
    }

    #[test]
    fn state_enumeration() {
        let m = create_machine();
        let states: Vec<_> = m.states().cloned().collect();
        assert!(states.contains(&TestState::Idle));
        assert!(states.contains(&TestState::Running));
        assert!(states.contains(&TestState::Paused));
        assert!(states.contains(&TestState::Stopped));
    }

    #[test]
    fn event_enumeration() {
        let m = create_machine();
        let events: Vec<_> = m.events().cloned().collect();
        assert_eq!(events.len(), 4);
        assert!(events.contains(&TestEvent::Start));
        assert!(events.contains(&TestEvent::Pause));
    }

    #[test]
    fn triggerable_events() {
        let m = create_machine();
        let triggerable: Vec<_> = m.triggerable_events().cloned().collect();
        assert_eq!(triggerable.as_slice(), &[TestEvent::Start]);
    }

    #[test]
    fn overwrite_transition() {
        let mut m = Machine::new(TestState::Idle);
        m.when(TestEvent::Start, TestState::Idle, TestState::Running);
        m.when(TestEvent::Start, TestState::Idle, TestState::Paused);

        m.trigger(&TestEvent::Start).unwrap();
        assert_eq!(*m.state(), TestState::Paused);
    }

    #[test]
    fn multiple_callbacks() {
        let mut m = create_machine();
        let counter = Rc::new(Cell::new(0));

        let c1 = counter.clone();
        m.on_enter_with(TestState::Running, move |_, _: &()| {
            c1.set(c1.get() + 1)
        });

        let c2 = counter.clone();
        m.on_enter_with(TestState::Running, move |_, _: &()| {
            c2.set(c2.get() + 1)
        });

        m.trigger(&TestEvent::Start).unwrap();
        assert_eq!(counter.get(), 2);
    }

    #[test]
    fn no_payload_callback() {
        let mut m = create_machine();
        let called = Rc::new(Cell::new(false));

        let c = called.clone();
        m.on_enter_with(TestState::Running, move |_, _: &()| c.set(true));

        m.trigger(&TestEvent::Start).unwrap();
        assert!(called.get());
    }

    #[test]
    fn invalid_payload_type() {
        let mut m = Machine::new(TestState::Idle);
        m.when(TestEvent::Start, TestState::Idle, TestState::Running);

        let called = Rc::new(Cell::new(false));
        let c = called.clone();
        m.on_enter_with(TestState::Running, move |_, _: &String| c.set(true));

        // Trigger with wrong payload type.
        m.trigger_with(&TestEvent::Start, &42i32).unwrap();
        assert!(!called.get());
    }

    #[test]
    fn on_no_payload_callback() {
        let mut m = create_machine();
        let fired = Rc::new(Cell::new(None));
        let fired_clone = fired.clone();
        // Register a no-payload callback for entering Running.
        m.on_enter(TestState::Running, move |evt| {
            fired_clone.set(Some(evt.clone()));
        });
        // Trigger Idle -> Running.
        m.trigger(&TestEvent::Start).unwrap();
        // Callback should have been called with Start event.
        assert_eq!(fired.take(), Some(TestEvent::Start));
    }

    #[test]
    fn on_no_payload_not_called_in_other_state() {
        let mut m = create_machine();
        let called = Rc::new(Cell::new(false));
        let called_clone = called.clone();
        // Register callback for Paused, but machine will go to Running.
        m.on_enter(TestState::Paused, move |_| {
            called_clone.set(true);
        });
        // Trigger Idle -> Running.
        m.trigger(&TestEvent::Start).unwrap();
        // Callback for Paused should not fire.
        assert!(!called.get());
    }

    #[test]
    fn callback_with_payload_not_called_when_no_payload() {
        let mut m = create_machine();
        let called = Rc::new(Cell::new(false));
        let called_clone = called.clone();
        // Register a payload callback for entering Running.
        m.on_enter_with(TestState::Running, move |_, _payload: &usize| {
            called_clone.set(true);
        });
        // Trigger Idle -> Running.
        m.trigger(&TestEvent::Start).unwrap();
        // Callback should have been called with Start event.
        assert!(!called.take());
    }

    #[test]
    fn callback_with_no_payload_called_when_payload() {
        let mut m = create_machine();
        let called = Rc::new(Cell::new(false));
        let called_clone = called.clone();
        // Register a payload callback for entering Running.
        m.on_enter(TestState::Running, move |_| {
            called_clone.set(true);
        });
        // Trigger Idle -> Running.
        m.trigger_with(&TestEvent::Start, &10usize).unwrap();
        // Callback should have been called with Start event.
        assert!(called.take());
    }

    #[test]
    fn any_state_with_payload_callback() {
        let mut m = create_machine();
        let count = Rc::new(Cell::new(0));
        let last = Rc::new(Cell::new(0u32));
        let ccount = count.clone();
        let clast = last.clone();
        // Register an on_any_with callback for u32 payloads.
        m.on_transition_with(move |_evt, amt: &u32| {
            ccount.set(ccount.get() + 1);
            clast.set(*amt);
        });

        // Trigger with matching payloads.
        m.trigger_with(&TestEvent::Start, &5u32).unwrap();
        m.trigger_with(&TestEvent::Pause, &10u32).unwrap();

        // The callback should have fired twice, and last payload should be 10.
        assert_eq!(count.get(), 2);
        assert_eq!(last.get(), 10);
    }

    #[test]
    fn any_state_with_wrong_payload_not_called() {
        let mut m = create_machine();
        let called = Rc::new(Cell::new(false));
        let c = called.clone();
        // Register an on_any_with callback expecting String payloads.
        m.on_transition_with(move |_, _: &String| {
            c.set(true);
        });

        // Trigger with a u32 payload instead.
        m.trigger_with(&TestEvent::Start, &5u32).unwrap();

        // The callback should not fire for the wrong payload type.
        assert!(!called.get());
    }
}
