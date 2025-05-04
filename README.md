# `nanomachine`

A minimal, flexible, and generic finite state machine (FSM) implementation in
Rust. Inspired by [MicroMachine](https://github.com/piotrmurach/micromachine),
this crate provides core FSM functionality with minimal boilerplate.

---

## Installation

Add this crate to your `Cargo.toml`:

```toml
[dependencies]
nanomachine = "0.1"
```

Then import it in your code:

```rust
use nanomachine::Machine;
```

---

## Usage

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

fn main() {
    // Create a new machine starting in the Locked state
    let mut fsm = Machine::new(State::Locked);

    // Define transitions
    fsm.when(Event::InsertCoin, State::Locked.clone(), State::Unlocked);
    fsm.when(Event::TurnKnob, State::Unlocked.clone(), State::Locked);

    // Register a callback when entering Unlocked
    fsm.on(State::Unlocked.clone(), |event, _payload: &()| {
        println!("Unlocked by event: {:?}", event);
    });

    // Trigger events
    assert!(fsm.trigger(&Event::InsertCoin));
    assert_eq!(*fsm.state(), State::Unlocked);

    assert!(fsm.trigger(&Event::TurnKnob));
    assert_eq!(*fsm.state(), State::Locked);
}
```

### Triggering with payloads

```rust
// You can attach data to transitions
fsm.on(State::Unlocked.clone(), |event, amount: &u32| {
    println!("Unlocked after {} cents by {:?}", amount, event);
});

// Pass a payload when triggering
fsm.trigger_with(&Event::InsertCoin, &50u32);
```
