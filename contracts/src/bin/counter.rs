/*
    The core idea is that everything in Solana is an account. 
    Think of accounts like files in an operating system.


    Data Storage: Accounts are primarily used to store data (state). This could be anything:

A user's wallet balance (SOL tokens).
The code of a deployed program (smart contract).
Data related to an NFT (metadata URI, owner).
State managed by a custom program (like a counter, game state, or DeFi pool information).

Addresses (Public Keys): Every account has a unique address, which is a public key (usually represented as a base58 string).
Ownership: Every account has an owner.

By default, user wallets (accounts holding SOL) are owned by the System Program. 
The System Program is a native Solana program that handles creating new accounts and transferring SOL.
Accounts that store the code of a program are owned by the BPF Loader Program.
Accounts created by your custom program to store its data are typically owned by your program. 
Only the owning program can modify the data within an account it owns 
(except for the System Program which can deduct lamports for rent or transaction fees).

Data: Accounts have a data field, which is just a byte array ([]u8). 
The interpretation of these bytes depends entirely on the program that owns and interacts with the account. 
Programs often use serialization formats like Borsh to structure this data.

Lamports: Accounts hold a balance of lamports (the smallest unit of SOL, 1 SOL=1,000,000,000 lamports). This balance is used for transaction fees and Rent.

Rent: Accounts consume storage space on the Solana cluster. To pay for this, accounts must either:

Pay rent periodically.
Maintain a minimum SOL balance to be considered "rent-exempt". This is the common approach for accounts intended to persist. 
The required balance depends on the size of the data stored in the account.

Executable Flag: Accounts that contain program code have an executable flag set to true. User accounts and data accounts have this set to false.

Stateless Programs: Solana programs themselves are stateless. 
The code stored in a program account doesn't change when it executes. 
All the state it operates on (like a counter's value, user balances in a custom token, etc.) is stored 
in separate data accounts  that are passed into the program during a transaction instruction.

Transactions and Instructions: When you send a transaction, you specify:

Which program you want to execute.
Which accounts will be involved in the instruction (read from or written to).
Any data needed by the instruction itself. 
The Solana runtime uses the list of accounts to potentially schedule transactions in parallel 
if they don't access the same accounts writeably.

Analogy:

Program Account: Like an executable file (.exe or binary) on your computer. It contains instructions but no user data. It's owned by the "operating system" (BPF Loader).
Data Account: Like a data file (.txt, .json, .dat) on your computer. It stores information. It's owned by the program that knows how to read/write it.
User Wallet Account: Like a special system file tracking your core currency, owned by the "operating system" (System Program).
Transaction Instruction: Like running a command: my_program.exe --input user_data.dat --output results.dat. You specify the program and the data files (accounts) it needs to work with.
*/


use solana_program::{
    account_info::{next_account_info, AccountInfo}, // Tools to handle accounts passed into the program
    entrypoint, // Macro to declare the program's entrypoint
    entrypoint::ProgramResult, // Standard result type for programs
    msg, // Macro for logging messages to the chain
    program_error::ProgramError, // Standard error type
    pubkey::Pubkey, // Solana public key type
    program::invoke, // For calling other programs (like System Program) - not used here but common
    system_instruction, // Instructions for the System Program - not used here but common
};
use borsh::{BorshDeserialize, BorshSerialize}; // For serializing/deserializing account data
use std::io::ErrorKind;


// Define the structure of the data we want to store in our data account
#[derive(BorshSerialize, BorshDeserialize, Debug)] // BorshSerialize and BorshDeserialize allow us to easily convert this struct to/from the raw byte array (account_data) stored in the account. why?

pub struct CounterAccount {
    pub counter: u64, // The actual counter value
}

// Define the instructions our program can accept
// In this simple case, we only have one: Increment
// More complex programs would have more variants
// Data for instructions is passed separately from accounts
// (We'll use an empty instruction data for simplicity here)
// enum CounterInstruction {
//     Increment,
//     Decrement, // Example of another instruction
//     Reset { value: u64 } // Example with data
// }


// Program entrypoint function
// Solana runtime calls this function when a transaction targets our program ID
entrypoint!(process_instruction);

// The main logic of our program
pub fn process_instruction(
    program_id: &Pubkey,      // Public key of OUR program account
    accounts: &[AccountInfo], // Array of accounts passed in by the transaction
    _instruction_data: &[u8], // Data passed specific to this instruction (we ignore it here)
) -> ProgramResult { // Must return ProgramResult (Ok or Err)
    msg!("Counter Program Entrypoint");

    // --- 1. Account Validation ---

    // Get the account iterator
    let accounts_iter = &mut accounts.iter();

    // Get the account we expect to store the counter data
    // The client building the transaction must pass this account
    let counter_account = next_account_info(accounts_iter)?;

    // Check 1: Is the counter_account owned by OUR program?
    // This is crucial! Only we should be able to modify the data structure
    // defined by `CounterAccount`.
    if counter_account.owner != program_id {
        msg!("Error: Counter account is not owned by this program");
        return Err(ProgramError::IncorrectProgramId);
    }

    // Check 2: Is the counter_account writable?
    // The transaction must mark this account as writable if we intend to change it.
    if !counter_account.is_writable {
         msg!("Error: Counter account must be writable");
         return Err(ProgramError::InvalidAccountData); // Using this error, adjust as needed
    }

    // --- 2. Instruction Logic ---

    // In a real program, you'd deserialize `_instruction_data` here to figure out
    // *what* action to take (e.g., Increment, Decrement, Reset).
    // For simplicity, we'll assume the only action is Increment.
    // let instruction = CounterInstruction::unpack(_instruction_data)?; // Example

    // --- 3. State Deserialization ---

    // Get the account's data buffer as a slice (mutable borrow because we checked is_writable)
    let mut account_data = counter_account.try_borrow_mut_data()?;

    // Deserialize the byte data into our `CounterAccount` struct
    // Use `try_from_slice` which handles errors gracefully.
    // If the account is new/uninitialized, this might fail.
    let mut counter_state = match CounterAccount::try_from_slice(&account_data) {
         Ok(state) => state,
         Err(e) => {
             // If the error is because the data is empty (uninitialized account),
             // initialize it. Otherwise, propagate the error.
             if e.kind() == ErrorKind::InvalidData || account_data.is_empty() {
                  msg!("Account not initialized. Initializing with counter = 0");
                  CounterAccount { counter: 0 }
             } else {
                 msg!("Error deserializing account data: {:?}", e);
                 return Err(ProgramError::InvalidAccountData);
             }
         }
     };

    // --- 4. Business Logic ---

    // Increment the counter
    counter_state.counter += 1; //where is this counter int defined?
    msg!("Counter incremented. New value: {}", counter_state.counter);

    // --- 5. State Serialization ---

    // Serialize the updated state back into the account's data buffer
     counter_state.serialize(&mut *account_data)?; // The `*` dereferences the mutable borrow RefMut<[u8]>

    msg!("Counter state saved.");
    Ok(()) // Indicate successful execution
}

// Note: This code doesn't handle creating the counter account itself.
// Account creation is usually done by the client (e.g., JavaScript code)
// using the System Program before calling this program's instruction.
// The client would:
// 1. Calculate the required rent-exempt reserve for the size of `CounterAccount`.
// 2. Create a new keypair for the counter account address.
// 3. Send a transaction with `SystemProgram.createAccount` instruction:
//    - Specify the new account's public key.
//    - Allocate space (using `std::mem::size_of::<CounterAccount>()`).
//    - Assign ownership to *this* program's ID (`program_id`).
//    - Transfer enough lamports for rent exemption.
// 4. Then, send a separate transaction calling *this* program's instruction,
//    passing the newly created counter account's public key in the `accounts` array.

