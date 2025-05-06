//! A comprehensive example of using nanomachine to model an e-commerce order
//! FSM: Created → Paid → Shipped → Delivered, with the option to Cancel at
//! various points.

use nanomachine::Machine;

#[derive(Debug, Clone, PartialEq, Eq, Hash)]
enum OrderState {
    Created,
    Paid,
    Shipped,
    Delivered,
    Cancelled,
}

#[derive(Debug, Clone, PartialEq, Eq, Hash)]
enum OrderEvent {
    Pay,     // With payload: amount in cents.
    Ship,    // With payload: tracking number.
    Deliver, // No payload.
    Cancel,  // With payload: reason string.
}

fn main() {
    // Create the machine in the Created state.
    let mut nano = Machine::new(OrderState::Created);

    // Define transitions one-by-one.
    nano.when(OrderEvent::Pay, OrderState::Created, OrderState::Paid);
    nano.when(OrderEvent::Ship, OrderState::Paid, OrderState::Shipped);
    nano.when(OrderEvent::Deliver, OrderState::Shipped, OrderState::Delivered);

    // Define Cancel for both Created and Paid using when_iter.
    nano.when_iter(
        OrderEvent::Cancel,
        [
            (OrderState::Created, OrderState::Cancelled),
            (OrderState::Paid, OrderState::Cancelled),
        ],
    );

    // ─── State-specific hooks ───────────────────────────────────────────────
    // On Paid: log payment amount.
    nano.on_enter_with(OrderState::Paid, |evt, amount: &u32| {
        println!("✔ Payment {:?}: {}¢ received", evt, amount);
    });

    // On Shipped: log tracking code.
    nano.on_enter_with(OrderState::Shipped, |_evt, tracking: &String| {
        println!("🚚 Order shipped! Tracking code: {}", tracking);
    });

    // On Delivered: confirmation.
    nano.on_enter_with(OrderState::Delivered, |_evt, _: &()| {
        println!("📦 Order delivered. Thank you!");
    });

    // On Cancelled: show cancellation reason.
    nano.on_enter_with(OrderState::Cancelled, |_evt, reason: &String| {
        println!("❌ Order cancelled: {}", reason);
    });

    // ─── Global hooks ───────────────────────────────────────────────────────
    // Fires on *every* transition (no payload).
    nano.on_transition(|evt| {
        println!("🔄 Transition via event: {:?}", evt);
    });

    // Fires on *every* transition *with* a u32 payload (e.g. payments).
    nano.on_transition_with(|evt, amount: &u32| {
        println!("🌐 Global saw {}¢ payment via {:?}", amount, evt);
    });

    // Fires on *every* transition *with* a String payload
    // (e.g. tracking codes or cancellation reasons).
    nano.on_transition_with(|evt, text: &String| {
        println!("🌐 Global saw string payload {:?}: {}", evt, text);
    });
    // Inspect machine metadata:
    println!("All states: {:?}", nano.states().cloned().collect::<Vec<_>>());
    println!("All events: {:?}", nano.events().cloned().collect::<Vec<_>>());
    println!(
        "Triggerable now ({:?}): {:?}",
        nano.state(),
        nano.triggerable_events().cloned().collect::<Vec<_>>()
    );
    println!();

    // Simulate a successful order:
    //    Pay with payload.
    nano.trigger_with(&OrderEvent::Pay, &2_500u32)
        .expect("Should transition to Paid");
    //    Ship with payload.
    nano.trigger_with(&OrderEvent::Ship, &"TRACK1234".to_string())
        .expect("Should transition to Shipped");
    //    Deliver without payload.
    nano.trigger(&OrderEvent::Deliver).expect("Should transition to Delivered");
    println!("\nFinal state: {:?}\n", nano.state());

    // Attempt an invalid transition (Pay after Delivered).
    match nano.trigger(&OrderEvent::Pay) {
        Ok(_) => println!("Unexpectedly succeeded!"),
        Err(e) => println!("Error trying to pay in {:?}: {}", nano.state(), e),
    }

    // Create a fresh order and cancel it early.
    let mut nano = Machine::new(OrderState::Created);
    nano.when_iter(
        OrderEvent::Cancel,
        [
            (OrderState::Created, OrderState::Cancelled),
            (OrderState::Paid, OrderState::Cancelled),
        ],
    );
    // Re-attach only the Cancel callback.
    nano.on_enter_with(OrderState::Cancelled, |_evt, reason: &String| {
        println!("Order #2 cancelled reason: {}", reason);
    });
    // Cancel with a payload.
    nano.trigger_with(
        &OrderEvent::Cancel,
        &"Customer changed mind".to_string(),
    )
    .unwrap();

    println!("Order #2 final: {:?}", nano.state());
}
