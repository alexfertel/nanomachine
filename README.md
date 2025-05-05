# `nanomachine`

A minimal, flexible, and generic finite state machine (FSM) implementation in
Rust, inspired by [MicroMachine](https://github.com/piotrmurach/micromachine).

No dependencies, `no_std` friendly, arbitrary state types with events and
callbacks.

## Installation

Add this crate to your `Cargo.toml`:

```toml
[dependencies]
nanomachine = "0.1"
```

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
  let mut fsm = Machine::new(State::Locked);

  // Define transitions.
  fsm.when(Event::InsertCoin, State::Locked, State::Unlocked);
  fsm.when(Event::TurnKnob, State::Unlocked, State::Locked);

  // Register a callback when entering Unlocked.
  fsm.on(State::Unlocked, |event, _payload: &()| {
    println!("Unlocked by event: {:?}", event);
  });

  assert!(fsm.trigger(&Event::InsertCoin).is_ok());
  assert_eq!(*fsm.state(), State::Unlocked);

  assert!(fsm.trigger(&Event::TurnKnob).is_ok());
  assert_eq!(*fsm.state(), State::Locked);

  // You can attach data to transitions.
  fsm.on(State::Unlocked, |event, amount: &u32| {
    println!("Unlocked after {} cents by {:?}", amount, event);
  });

  // Pass a payload when triggering.
  fsm.trigger_with(&Event::InsertCoin, &50u32);
}
```
