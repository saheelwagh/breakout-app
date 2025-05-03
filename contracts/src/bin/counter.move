// Filename: sources/my_counter.move
// Sui Move module defining a simple Counter object

// Define the module within an address space (e.g., 0x0 represents a placeholder during development/testing)
// When deployed, this will be the address of the package publisher.
module my_package::my_counter {

    // Import necessary modules from the Sui framework
    use sui::object::{Self, UID}; // For object creation and ID handling
    use sui::transfer;           // For transferring object ownership
    use sui::tx_context::{Self, TxContext}; // Provides transaction context (sender, etc.)

    // === Object Definition ===

    /// Represents a counter object on the Sui network.
    /// It has the 'key' ability, meaning it can be stored directly in Sui's global storage
    /// and can be transferred between owners.
    struct Counter has key {
        id: UID,        // Unique identifier for every Sui object, automatically managed
        counter: u64,   // The actual counter value
    }

    // === Entry Functions ===
    // These functions can be called directly as part of a Sui transaction.

    /// Creates a new Counter object and transfers it to the transaction sender.
    /// `ctx: &mut TxContext` provides information about the transaction context,
    /// including the sender's address. It's required for functions that
    /// create or mutate objects or transfer ownership.
    public entry fun create(ctx: &mut TxContext) {
        // Create the Counter object instance
        let counter_object = Counter {
            id: object::new(ctx), // Generate a new unique ID for this object within this transaction context
            counter: 0,           // Initialize the counter to zero
        };

        // Transfer the newly created `counter_object` to the sender of the transaction.
        // This makes the sender the owner of this specific Counter instance.
        transfer::transfer(counter_object, tx_context::sender(ctx));
    }

    /// Increments the value of a given Counter object.
    /// `counter: &mut Counter` takes a mutable reference to a Counter object.
    /// The Sui runtime ensures that the transaction sender has the necessary rights
    /// (e.g., ownership) to provide this mutable reference.
    /// `_ctx: &mut TxContext` is included because this function mutates state,
    /// even though we don't explicitly use `ctx` here.
    public entry fun increment(counter: &mut Counter, _ctx: &mut TxContext) {
        // Directly access and increment the 'counter' field of the object.
        // No serialization/deserialization needed.
        counter.counter = counter.counter + 1;
    }

    // === Public View Function ===
    // These functions can be called off-chain (e.g., via RPC) to read state
    // without submitting a transaction. They cannot modify state.

    /// Returns the current value of the counter.
    /// Takes an immutable reference `&Counter`.
    public fun value(counter: &Counter): u64 {
        counter.counter
    }

    // === Tests (Optional but recommended) ===
    // Move includes a built-in test framework.

    #[test_only] // Indicates this code is only for testing
    module my_package::my_counter_tests {
        // Import items needed for tests
        use my_package::my_counter::{Self, Counter};
        use sui::test_scenario::{Self, Scenario, next_tx, ctx}; // Sui testing framework
        use sui::object;

        // Define a test address (can be anything for testing)
        const TEST_SENDER: address = @0xABC;

        #[test]
        fun test_create_and_increment() {
            // Initialize a test scenario
            let mut scenario = test_scenario::begin(TEST_SENDER);

            // === Test Create ===
            {
                // Simulate the 'create' transaction
                next_tx(&mut scenario, TEST_SENDER);
                my_counter::create(ctx(&mut scenario));
            }; // Transaction block ends here

            // === Test Increment ===
            {
                // Simulate the 'increment' transaction
                next_tx(&mut scenario, TEST_SENDER);

                // Get the Counter object created in the previous transaction from the scenario
                // We need to know its type (`Counter`) to retrieve it.
                let mut counter_object = test_scenario::take_owned<Counter>(&mut scenario);

                // Call the increment function
                my_counter::increment(&mut counter_object, ctx(&mut scenario));

                // Check the value using the view function
                assert!(my_counter::value(&counter_object) == 1, 0);

                // Put the object back into the scenario's state (important!)
                test_scenario::return_owned(&mut scenario, counter_object);
            }; // Transaction block ends here

            // Clean up the scenario
            test_scenario::end(scenario);
        }
    }
}
```

**How to Interact (Conceptual):**

1.  **Publish:** You'd publish this `my_package` to the Sui network. This creates a Package object.
2.  **Create Counter:** A user sends a transaction calling the `my_package::my_counter::create` function. This transaction creates a new `Counter` object, and the user becomes its owner. They pay a one-time storage fee.
3.  **Increment Counter:** The user (owner) sends another transaction calling `my_package::my_counter::increment`, passing the specific `ObjectID` of *their* `Counter` object as an argument. The Move function receives a mutable reference (`&mut Counter`), increments the field, and the change is saved.
4.  **Read Value:** Anyone can query the state of a specific `Counter` object (if they know its `ObjectID`) using an RPC call to the `my_counter::value` function without sending a transaction.

This Move example showcases a more object-oriented approach where state and logic are tightly coupled within typed objects, contrasting with Solana's model of operating on generic byte arrays stored in accoun