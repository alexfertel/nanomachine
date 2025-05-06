//! A comprehensive example of using nanomachine callbacks.

use nanomachine::Machine;

fn main() {
    let mut nano = Machine::new("locked");

    // Define valid transitions.
    nano.when("insert coin", "locked", "unlocked");
    nano.when("turn knob", "unlocked", "locked");

    // State-specific callbacks without payload.
    nano.on_enter("unlocked", |e| println!("Unlocked via {:?}", e));
    nano.on_enter("locked", |e| println!("Locked via {:?}", e));

    // State-specific callback with a u32 payload.
    nano.on_enter_with("unlocked", |e, amount: &u32| {
        println!("Received {} cents via {:?}", amount, e);
    });

    // Global callbacks.
    nano.on_transition(|e| println!("Global - event {:?}", e));
    nano.on_transition_with(|e, amount: &u32| {
        println!("Global - saw {} cents via {:?}", amount, e);
    });
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
    nano.trigger_with(&"turn knob", &"voucher".to_string()).unwrap();

    // Prints:
    //   Unlocked via "insert coin"
    //   Received 50 cents via "insert coin"
    //   Global - event "insert coin"
    //   Global - saw 50 cents via "insert coin"
    nano.trigger_with(&"insert coin", &50u32).unwrap();

    // Final state: "unlocked"
    println!("Final state: {:?}", nano.state());
}
