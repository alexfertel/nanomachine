    ███╗░░██╗░█████╗░███╗░░██╗░█████╗░███╗░░░███╗░█████╗░░█████╗░██╗░░██╗██╗███╗░░██╗███████╗
    ████╗░██║██╔══██╗████╗░██║██╔══██╗████╗░████║██╔══██╗██╔══██╗██║░░██║██║████╗░██║██╔════╝
    ██╔██╗██║███████║██╔██╗██║██║░░██║██╔████╔██║███████║██║░░╚═╝███████║██║██╔██╗██║█████╗░░
    ██║╚████║██╔══██║██║╚████║██║░░██║██║╚██╔╝██║██╔══██║██║░░██╗██╔══██║██║██║╚████║██╔══╝░░
    ██║░╚███║██║░░██║██║░╚███║╚█████╔╝██║░╚═╝░██║██║░░██║╚█████╔╝██║░░██║██║██║░╚███║███████╗
    ╚═╝░░╚══╝╚═╝░░╚═╝╚═╝░░╚══╝░╚════╝░╚═╝░░░░░╚═╝╚═╝░░╚═╝░╚════╝░╚═╝░░╚═╝╚═╝╚═╝░░╚══╝╚══════╝

# `nanomachine`

A minimal, flexible, and generic finite state machine (FSM) implementation in
Rust, inspired by [MicroMachine](https://github.com/piotrmurach/micromachine).

No dependencies, `no_std`, arbitrary state types, events, and callbacks. The API
is intentionally small and is planned to remain as such. They are state
machines, and that's it.

If an FSM needs to be implemented for a piece of application logic and there is
no time to learn a DSL or design many types, `nanomachine` can get the job done
with a few lines.

If guarded transitions or hierarchical states are needed, this crate is not the
right tool.

Note that machines are not thread-safe; this may change in future versions.

## Installation

Add this crate to the `Cargo.toml`:

```toml
[dependencies]
nanomachine = "0.1"
```

## Usage

This state machine:

```
╔══════════╗    Insert Coin    ╔════════════╗
║          ║══════════════════>║            ║
║  Locked  ║                   ║  Unlocked  ║
║          ║<══════════════════║            ║
╚══════════╝     Turn Knob     ╚════════════╝
```

would be represented as:

```rust
use nanomachine::Machine;

let mut nano = Machine::new("locked");

// Define transitions.
nano.when("insert coin", "locked", "unlocked");
nano.when("turn knob", "unlocked", "locked");

// Register a callback.
nano.on_enter("unlocked", |event| {
    println!("Unlocked by event: {:?}", event);
});

nano.trigger(&"insert coin"); // <-- prints: Unlocked by event: insert coin
assert_eq!(*nano.state(), "unlocked");

nano.trigger(&"turn knob");
assert_eq!(*nano.state(), "locked");

// Add inputs to transitions.
nano.on_enter_with("unlocked", |_, payload: &u32| {
    println!("Unlocked after {} cents", payload);
});
```

A payload is an arbitrary value we can pass to callbacks when triggering an
event. Payloads can be of any type:

```rust
nano.on_enter_with("locked", |_, payload: &(&str, f32)| {
    println!("Locked with a fancy payload: {:?}", payload);
});

nano.trigger_with(&"turn knob", &("fancy", 42f32)); // <- prints: Unlocked after 50 cents
```

Note that if we hadn't defined the float as a `42f32`, but as `42.`, we couldn't
know that it matches a type of `(&str, f32)`, so the callback wouldn't have been
called.

The state can be any type that implements `Eq + Hash + Clone`:

```rust
use nanomachine::Machine;

#[derive(Debug, Clone, PartialEq, Eq, Hash)]
enum State {
  Locked,
  Unlocked,
}

#[derive(Debug, Clone, PartialEq, Eq, Hash)]
enum Event {
  InsertCoin,
  TurnKnob,
}

let mut nano = Machine::new(State::Locked);

nano.when(Event::InsertCoin, State::Locked, State::Unlocked);
nano.when(Event::TurnKnob, State::Unlocked, State::Locked);

nano.on_enter(State::Unlocked, |event| {
    println!("Unlocked by event: {:?}", event);
});

nano.trigger(&Event::InsertCoin);
assert_eq!(*nano.state(), State::Unlocked);

nano.trigger(&Event::TurnKnob);
assert_eq!(*nano.state(), State::Locked);

nano.on_enter_with(State::Unlocked, |_, payload: &u32| {
    println!("Unlocked after {} cents", payload);
});

nano.trigger_with(&Event::InsertCoin, &50u32);
```

If the event fired does not trigger a transition, then an error with an
appropriate reason is returned.

```rust
nano.state(); // <- Locked

nano.trigger(&Event::TurnKnob);   // <- Err(MachineError::StateInvalid)
nano.trigger(&Event::Insertcoin); // <- Unlocked
```

We can also list all possible events or states:

```rust
// All possible states.
nano.states().collect::<Vec<_>>(); // vec![&Locked, &Unlocked]

// All events triggerable from the current state.
nano.triggerable_events().collect::<Vec<_>>(); // vec![&InsertCoin]

// All events.
nano.events().collect::<Vec<_>>(); // vec![&InsertCoin, &TurnKnob]
```

### Callbacks

We can register callbacks with no payload that get triggered when entering a
given state:

```rust
nano.on_enter(State::Unlocked, |event| {
    println!("Unlocked by event: {:?}", event);
});
```

Additionally, it can be useful to register callbacks that expect some payload:

```rust
nano.on_enter_with(State::Unlocked, |_, payload: &u32| {
    println!("Unlocked after {} cents", payload);
});
```

We may also want to register callbacks to be fired on every transition:

```rust
// Prints after every `trigger` call.
nano.on_transition(|e| {
    println!("Global - event {:?}", e);
});

// Prints after every `trigger_with` call with payload of the appropriate type.
nano.on_transition_with(|e, payload: &u32| {
    println!("Global - saw {} cents via {:?}", payload, e);
});
```

Note that the order in which the callbacks are called is *not* deterministic, in
other words, the order in which they are registered is irrelevant.

Keep in mind that only callbacks with payload of the appropriate type will be
triggered. Global callbacks with no payload *always* get triggered.

```rust
// Always triggered.
nano.on_transition(|e| println!("Global - event {:?}", e));

// Triggered on every transition when the payload is a `u32`.
nano.on_transition_with(|e, payload: &u32| {
    println!("Global - saw {} cents via {:?}", payload, e);
});

// Triggered on every transition when the payload is a `String`.
nano.on_transition_with(|e, msg: &String| {
    println!("Global - saw \"{}\" via {:?}", msg, e);
});

// Prints:
//   Unlocked via "insert coin"
//   Global - event "insert coin"
nano.trigger(&"insert coin").unwrap();

// Prints:
//   Locked via "turn knob"
//   Global - event "turn knob"
//   Global - saw "voucher" via "turn knob"
nano.trigger_with(&"turn knob", &"voucher".to_string())
    .unwrap();

// Prints:
//   Unlocked via "insert coin"
//   Received 50 cents via "insert coin"
//   Global - event "insert coin"
//   Global - saw 50 cents via "insert coin"
nano.trigger_with(&"insert coin", &50u32).unwrap();
```
