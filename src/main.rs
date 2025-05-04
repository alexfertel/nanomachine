use nanomachine::Machine;

fn main() {
    // machine = MicroMachine.new(:new) # Initial state.
    //
    // machine.when(:confirm, :new => :confirmed)
    // machine.when(:ignore, :new => :ignored)
    // machine.when(:reset, :confirmed => :new, :ignored => :new)

    // machine.trigger(:confirm)  #=> true
    // machine.state              #=> :confirmed

    // machine.trigger(:ignore)   #=> false
    // machine.state              #=> :confirmed

    // machine.trigger(:reset)    #=> true
    // machine.state              #=> :new

    // machine.trigger(:ignore)   #=> true
    // machine.state              #=> :ignored

    let mut machine = Machine::new("new");
    machine.when("confirm", "new", "confirmed");
    machine.when("ignore", "new", "ignored");
    machine.when("reset", "confirmed", "new");
    machine.when("reset", "ignored", "new");

    machine.trigger(&"confirm");
    println!("{}", machine.state());

    machine.trigger(&"ignore");
    println!("{}", machine.state());

    machine.trigger(&"reset");
    println!("{}", machine.state());

    machine.trigger(&"ignore");
    println!("{}", machine.state());

    machine.trigger(&"reset");
    println!("{}", machine.state());

    machine.trigger(&"confirm");
    println!("{}", machine.state());

    machine.trigger(&"reset");
    println!("{}", machine.state());

    println!("{:?}", machine.states().collect::<Vec<_>>());
    println!("{:?}", machine.events().collect::<Vec<_>>());
    println!("{:?}", machine.triggerable_events().collect::<Vec<_>>());
}
