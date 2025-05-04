//! A generic finite state machine implementation.
//!
//! This module provides a `Machine` struct that represents a finite state
//! machine (FSM) with generic state (`S`) and event (`E`) types. You can define
//! transitions between states based on events, register callbacks for entering
//! specific states or for any transition, and trigger events with optional
//! payloads.
#![warn(clippy::perf, clippy::pedantic, missing_docs)]

extern crate alloc;

use alloc::{rc::Rc, vec::Vec};
use core::{any::Any, hash::Hash};
use hashbrown::{HashMap, HashSet};

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
        self.transitions
            .entry(event)
            .or_default()
            .insert(state, new_state);
    }

    /// Define multiple transitions for a single event.
    ///
    /// The `mapping` iterator should yield `(from_state, to_state)` pairs.
    pub fn when_iter<I>(&mut self, event: E, mapping: I)
    where
        I: IntoIterator<Item = (S, S)>,
    {
        let entry = self.transitions.entry(event).or_default();
        entry.extend(mapping);
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

    /// Register a callback to fire when entering the given `state`.
    ///
    /// The callback takes the triggering event and a payload of type `P`. It
    /// will only be invoked if the payload downcasts to `P` successfully.
    pub fn on<P, F>(&mut self, state: S, callback: F)
    where
        P: 'static,
        F: Fn(E, &P) + 'static,
    {
        let callback = Self::wrap_callback(callback);
        self.callbacks
            .entry(Trigger::State(state))
            .or_default()
            .push(callback);
    }

    /// Register a callback to fire on any state transition.
    ///
    /// Works similarly to `on`, but the callback runs regardless of the
    /// specific state.
    pub fn on_any<P, F>(&mut self, callback: F)
    where
        P: 'static,
        F: Fn(E, &P) + 'static + Clone,
    {
        let callback = Self::wrap_callback(callback);
        self.callbacks
            .entry(Trigger::AnyState)
            .or_default()
            .push(callback);
    }

    /// Trigger `event` without a payload.
    ///
    /// Returns `true` if a valid transition occurred, `false` otherwise.
    pub fn trigger(&mut self, event: &E) -> bool {
        self.trigger_with(event, &())
    }

    /// Trigger `event` with a payload of type `P`.
    ///
    /// Returns `true` if a valid transition occurred and callbacks were
    /// invoked, or `false` if no transition was defined for the current state
    /// and event.
    pub fn trigger_with<P>(&mut self, event: &E, payload: &P) -> bool
    where
        P: 'static,
    {
        let maybe_state = self
            .transitions
            .get(event)
            .map(|state_map| state_map.get(&self.state));
        let Some(Some(next)) = maybe_state else {
            return false;
        };

        self.state = next.clone();

        let state_cbs = self.callbacks.get(&Trigger::State(self.state.clone()));
        let any_cbs = self.callbacks.get(&Trigger::AnyState);
        for cb in state_cbs.into_iter().chain(any_cbs.into_iter()).flatten() {
            cb(event.clone(), payload as &dyn Any);
        }

        true
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

#[cfg(test)]
mod tests {
    use super::*;
    use std::cell::Cell;

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
            vec![
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
        assert!(m.trigger(&TestEvent::Start));
        assert_eq!(*m.state(), TestState::Running);
    }

    #[test]
    fn invalid_transition() {
        let mut m = create_machine();
        assert!(!m.trigger(&TestEvent::Pause));
        assert_eq!(*m.state(), TestState::Idle);
    }

    #[test]
    fn state_specific_callback() {
        let mut m = create_machine();
        let callback_called = Rc::new(Cell::new(false));

        let cc = callback_called.clone();
        m.on(TestState::Running, move |_, _: &()| {
            cc.set(true);
        });

        m.trigger(&TestEvent::Start);
        assert!(callback_called.get());
    }

    #[test]
    fn any_state_callback() {
        let mut m = create_machine();
        let callback_count = Rc::new(Cell::new(0));

        let cc = callback_count.clone();
        m.on_any(move |_, _: &()| {
            cc.set(cc.get() + 1);
        });

        m.trigger(&TestEvent::Start);
        m.trigger(&TestEvent::Pause);
        assert_eq!(callback_count.get(), 2);
    }

    #[test]
    fn payload_handling() {
        let mut m = Machine::new(TestState::Idle);
        let payload_received = Rc::new(Cell::new(None));

        m.when(TestEvent::Start, TestState::Idle, TestState::Running);

        let pr = payload_received.clone();
        m.on(TestState::Running, move |_, p: &String| {
            pr.set(Some(p.clone()));
        });

        m.trigger_with(&TestEvent::Start, &"test payload".to_string());
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
        assert_eq!(triggerable, vec![TestEvent::Start]);
    }

    #[test]
    fn overwrite_transition() {
        let mut m = Machine::new(TestState::Idle);
        m.when(TestEvent::Start, TestState::Idle, TestState::Running);
        m.when(TestEvent::Start, TestState::Idle, TestState::Paused);

        m.trigger(&TestEvent::Start);
        assert_eq!(*m.state(), TestState::Paused);
    }

    #[test]
    fn multiple_callbacks() {
        let mut m = create_machine();
        let counter = Rc::new(Cell::new(0));

        let c1 = counter.clone();
        m.on(TestState::Running, move |_, _: &()| c1.set(c1.get() + 1));

        let c2 = counter.clone();
        m.on(TestState::Running, move |_, _: &()| c2.set(c2.get() + 1));

        m.trigger(&TestEvent::Start);
        assert_eq!(counter.get(), 2);
    }

    #[test]
    fn no_payload_callback() {
        let mut m = create_machine();
        let called = Rc::new(Cell::new(false));

        let c = called.clone();
        m.on(TestState::Running, move |_, _: &()| c.set(true));

        m.trigger(&TestEvent::Start);
        assert!(called.get());
    }

    #[test]
    fn invalid_payload_type() {
        let mut m = Machine::new(TestState::Idle);
        m.when(TestEvent::Start, TestState::Idle, TestState::Running);

        let called = Rc::new(Cell::new(false));
        let c = called.clone();
        m.on(TestState::Running, move |_, _: &String| c.set(true));

        // Trigger with wrong payload type
        m.trigger_with(&TestEvent::Start, &42i32);
        assert!(!called.get());
    }
}
