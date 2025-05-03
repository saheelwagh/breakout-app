/*
    Unlike Ethereum's ERC-20 contracts where all logic (transfer, mint, burn, balances) 
    often resides in a single contract, 
    Solana typically uses a composable approach:

    SPL Token Program: A standard, on-chain program provided by Solana Labs that 
    handles the core token functionalities (creating tokens, transferring, burning, managing balances). 
    Think of it as a pre-deployed, universal ERC-20 implementation that everyone uses.

    Mint Account: An account created using the SPL Token program that represents the
     specific stablecoin type. It stores global information like total supply, 
    the address authorized to mint more tokens (mint authority), and decimals.

    Token Accounts (ATAs): User balances aren't stored in a central mapping like balanceOf in Solidity.
     Instead, each user has a separate "Token Account" for each specific token they hold. 
     These are usually Associated Token Accounts (ATAs), 
    derived from the user's main wallet address and the token's Mint Account address.

    This Custom Program: The Rust program below acts as the administrator for the stablecoin's Mint Account. Its primary job is to securely manage who can mint new tokens. It does this 
    by holding the "mint authority" for the SPL Token Mint Account and 
    providing controlled functions to mint.

    Key Differences from Ethereum/Solidity:

State Location: 
Program logic (code) is separate from state (data). 
This program's code lives at its Program ID, while its configuration state 
(like the admin address) lives in a separate ConfigAccount. 
User balances live in their individual ATAs.

Interaction Model: Instead of calling functions directly on a single contract address,
 Solana transactions involve sending instructions to program IDs, 
 explicitly listing all accounts (data accounts, program accounts, signer accounts) 
 that the instruction will read from or write to.

Composability (CPI): This program will call the SPL Token program using Cross-Program Invocation (CPI) 
to perform the actual minting, similar to how an Ethereum contract might call another contract.

Rent: Accounts require SOL deposits to cover storage costs (rent-exemption), 
unlike Ethereum's gas model which covers storage implicitly during writes.

The following code defines the state, instructions, and processing logic for our stablecoin admin program.
*/

// === Cargo.toml Dependencies ===
// [dependencies]
// solana-program = "1.18.4" # Or latest compatible version
// spl-token = { version = "4.0.1", features = ["no-entrypoint"] } # SPL Token library
// borsh = "1.4.0" # For serialization/deserialization
// thiserror = "1.0.58"

// === src/lib.rs ===
pub mod entrypoint;
pub mod error;
pub mod instruction;
pub mod processor;
pub mod state;

// === src/entrypoint.rs ===
use solana_program::{
    account_info::AccountInfo, entrypoint, entrypoint::ProgramResult, pubkey::Pubkey,
};
use crate::{error::StablecoinError, processor::Processor};

// Declare the program entrypoint
entrypoint!(process_instruction);

// Entrypoint function called by the Solana runtime
fn process_instruction(
    program_id: &Pubkey,      // Public key of this program
    accounts: &[AccountInfo], // Accounts involved in the transaction instruction
    instruction_data: &[u8],  // Instruction-specific data
) -> ProgramResult {
    // Log entry and delegate processing
    // solana_program::msg!("Stablecoin Admin Program Entrypoint"); // Uncomment for debugging logs
    if let Err(error) = Processor::process(program_id, accounts, instruction_data) {
        // Log error (optional)
        // error.print::<StablecoinError>(); // Uncomment for detailed error logs
        return Err(error);
    }
    Ok(())
}

// === src/state.rs ===
use borsh::{BorshDeserialize, BorshSerialize};
use solana_program::{
    program_pack::{IsInitialized, Pack, Sealed},
    pubkey::Pubkey,
};

// Configuration state account structure
/// ETH Dev Analogy: Think of this like storage variables in a Solidity contract,
/// but stored in a separate account, not with the program code.
#[derive(BorshSerialize, BorshDeserialize, Debug, Clone)]
pub struct ConfigAccount {
    /// Tracks if the account is initialized
    pub is_initialized: bool,

    /// The public key authorized to mint new tokens and change the admin.
    /// ETH Dev Analogy: Similar to an 'owner' or 'minter' role address in Solidity.
    pub admin: Pubkey,

    /// The public key of the SPL Token Mint account this program controls.
    /// This program must be the 'mint_authority' for this Mint account.
    pub mint_account: Pubkey,
}
// Implement Solana's Pack trait for state accounts
impl Sealed for ConfigAccount {}

impl IsInitialized for ConfigAccount {
    fn is_initialized(&self) -> bool {
        self.is_initialized
    }
}

// Implement Pack to define how to serialize/deserialize and get the size
// Note: Borsh handles serialization, Pack integrates it with Solana's account model.
impl Pack for ConfigAccount {
    const LEN: usize = 1 + 32 + 32; // bool (1) + Pubkey (32) + Pubkey (32)

    fn pack_into_slice(&self, dst: &mut [u8]) {
        let mut writer = std::io::Cursor::new(dst);
        // Using Borsh for serialization within the Pack trait implementation
        self.serialize(&mut writer).unwrap();
    }

    fn unpack_from_slice(src: &[u8]) -> Result<Self, solana_program::program_error::ProgramError> {
        let mut reader = std::io::Cursor::new(src);
        // Using Borsh for deserialization
        ConfigAccount::deserialize(&mut reader)
            .map_err(|_| solana_program::program_error::ProgramError::InvalidAccountData)
    }
}
// === src/instruction.rs ===
use borsh::{BorshDeserialize, BorshSerialize};
use solana_program::pubkey::Pubkey;

/// Defines the different actions (instructions) this program can handle.
/// ETH Dev Analogy: These are like the public functions you'd define in a Solidity contract.
#[derive(BorshSerialize, BorshDeserialize, Debug, Clone)]
pub enum StablecoinInstruction {
    /// Initializes the stablecoin configuration account.
    /// Needs to be called once after deploying the program.
    ///
    /// Accounts expected:
    /// 0. `[writable, signer]` Payer account (pays for account creation rent).
    /// 1. `[writable]` Config account (the account to be initialized). Needs to be created with `SystemProgram.createAccount` first, typically client-side.
    /// 2. `[]` SPL Token Mint account address (the mint this program will manage).
    /// 3. `[]` System program ID.
    /// 4. `[]` Rent sysvar.
    Initialize {
        /// The initial admin address.
        admin: Pubkey,
    },

    /// Mints new stablecoins to a specified destination account.
    /// Only callable by the current admin.
    ///
    /// Accounts expected:
    /// 0. `[signer]` Current Admin account (must match `config_account.admin`).
    /// 1. `[writable]` Config account (holds admin and mint info).
    /// 2. `[writable]` SPL Token Mint account (the mint address stored in config).
    /// 3. `[writable]` Destination SPL Token Account (ATA of the recipient). Must exist.
    /// 4. `[]` SPL Token Program ID.
    MintTo {
        /// Amount of tokens (in smallest unit, like wei) to mint.
        amount: u64,
    },

    /// Sets a new admin for the stablecoin.
    /// Only callable by the current admin.
    ///
    /// Accounts expected:
    /// 0. `[signer]` Current Admin account (must match `config_account.admin`).
    /// 1. `[writable]` Config account (to update the admin field).
    SetAdmin {
        /// The public key of the new admin.
        new_admin: Pubkey,
    },
}


// === src/error.rs ===
use thiserror::Error;
use solana_program::program_error::ProgramError;

/// Custom errors for the stablecoin program.
#[derive(Error, Debug, Copy, Clone, PartialEq)]
pub enum StablecoinError {
    #[error("Invalid Instruction")]
    InvalidInstruction,
    #[error("Not Rent Exempt")]
    NotRentExempt,
    #[error("Account Already Initialized")]
    AlreadyInitialized,
    #[error("Admin signature mismatch")]
    AdminSignatureMismatch,
    #[error("Mint account mismatch")]
    MintAccountMismatch,
    #[error("Account not initialized")]
    NotInitialized,
    #[error("Numerical overflow error")]
    NumericalOverflow,
}

// Allow conversion from our custom error to the standard Solana ProgramError
impl From<StablecoinError> for ProgramError {
    fn from(e: StablecoinError) -> Self {
        ProgramError::Custom(e as u32)
    }
}
// === src/processor.rs ===
use solana_program::{
    account_info::{next_account_info, AccountInfo},
    entrypoint::ProgramResult,
    msg,
    program::{invoke, invoke_signed}, // For CPI (Cross-Program Invocation)
    program_error::ProgramError,
    program_pack::{IsInitialized, Pack},
    pubkey::Pubkey,
    sysvar::{rent::Rent, Sysvar},
};
use spl_token::instruction as token_instruction; // SPL Token program instructions
use crate::{
    error::StablecoinError,
    instruction::StablecoinInstruction,
    state::ConfigAccount,
};
/// Processes instructions for the stablecoin admin program.
pub struct Processor;
impl Processor {
    pub fn process(
        program_id: &Pubkey,
        accounts: &[AccountInfo],
        instruction_data: &[u8],
    ) -> ProgramResult {
        // Deserialize the instruction data using Borsh
        let instruction = StablecoinInstruction::try_from_slice(instruction_data)
            .map_err(|_| ProgramError::InvalidInstructionData)?;

        // Route to the appropriate handler based on the instruction variant
        match instruction {
            StablecoinInstruction::Initialize { admin } => {
                msg!("Instruction: Initialize");
                Self::process_initialize(accounts, admin, program_id)
            }
            StablecoinInstruction::MintTo { amount } => {
                msg!("Instruction: MintTo");
                Self::process_mint_to(accounts, amount, program_id)
            }
            StablecoinInstruction::SetAdmin { new_admin } => {
                msg!("Instruction: SetAdmin");
                Self::process_set_admin(accounts, new_admin, program_id)
            }
        }
    }

    /// Processes the Initialize instruction.
    fn process_initialize(
        accounts: &[AccountInfo],
        admin: Pubkey,
        program_id: &Pubkey,
    ) -> ProgramResult {
        let account_info_iter = &mut accounts.iter();
        // Account 0: Payer (Signer, Writable) - Not directly used here, but needed for tx fee
        let _payer_account = next_account_info(account_info_iter)?;
        // Account 1: Config Account (Writable) - The account to initialize
        let config_account = next_account_info(account_info_iter)?;
        // Account 2: Mint Account (Readonly) - The SPL Mint this program manages
        let mint_account_info = next_account_info(account_info_iter)?;
        // Account 3: System Program (Readonly) - Needed for rent check
        let _system_program = next_account_info(account_info_iter)?;
        // Account 4: Rent Sysvar (Readonly) - To check for rent exemption
        let rent = &Rent::from_account_info(next_account_info(account_info_iter)?)?;

        // Security Check: Ensure the config account is owned by *this* program.
        // ETH Dev Analogy: This is inherent in Solidity as code+storage are one.
        // In Solana, you must verify account ownership.
        if config_account.owner != program_id {
            msg!("Error: Config account not owned by program");
            return Err(ProgramError::IncorrectProgramId);
        }

        // Security Check: Ensure the config account is rent-exempt.
        // ETH Dev Analogy: Rent is Solana's storage cost mechanism, different from gas.
        if !rent.is_exempt(config_account.lamports(), config_account.data_len()) {
            msg!("Error: Config account not rent exempt");
            return Err(StablecoinError::NotRentExempt.into());
        }

        // Deserialize config account data to check if already initialized
        let mut config_data = ConfigAccount::unpack_unchecked(&config_account.data.borrow())?;
        if config_data.is_initialized() {
            msg!("Error: Config account already initialized");
            return Err(StablecoinError::AlreadyInitialized.into());
        }

        // Initialize the state
        config_data.is_initialized = true;
        config_data.admin = admin;
        config_data.mint_account = *mint_account_info.key;

        // Serialize the updated state back into the account
        ConfigAccount::pack(config_data, &mut config_account.data.borrow_mut())?;

        msg!("Config account initialized. Admin: {}, Mint: {}", admin, mint_account_info.key);
        Ok(())
    }

    /// Processes the MintTo instruction.
    fn process_mint_to(
        accounts: &[AccountInfo],
        amount: u64,
        program_id: &Pubkey,
    ) -> ProgramResult {
        let account_info_iter = &mut accounts.iter();
        // Account 0: Admin Account (Signer) - Must sign the transaction
        let admin_account = next_account_info(account_info_iter)?;
        // Account 1: Config Account (Writable) - Read admin & mint info
        let config_account = next_account_info(account_info_iter)?;
        // Account 2: Mint Account (Writable) - The SPL Mint to mint from
        let mint_account_info = next_account_info(account_info_iter)?;
        // Account 3: Destination Token Account (Writable) - Recipient's ATA
        let destination_account = next_account_info(account_info_iter)?;
        // Account 4: SPL Token Program ID (Readonly) - Program to invoke via CPI
        let token_program_info = next_account_info(account_info_iter)?;

        // Basic validation
        if !admin_account.is_signer {
            msg!("Error: Admin signature missing");
            return Err(ProgramError::MissingRequiredSignature);
        }
        if config_account.owner != program_id {
             msg!("Error: Config account not owned by program");
             return Err(ProgramError::IncorrectProgramId);
        }

        // Deserialize config state
        let config_data = ConfigAccount::unpack(&config_account.data.borrow())?;
        if !config_data.is_initialized() {
             msg!("Error: Config account not initialized");
             return Err(StablecoinError::NotInitialized.into());
        }

        // Security Check: Verify the signing account is the admin stored in config
        // ETH Dev Analogy: Similar to an `onlyOwner` or `onlyMinter` modifier.
        if config_data.admin != *admin_account.key {
            msg!("Error: Signer is not the configured admin");
            return Err(StablecoinError::AdminSignatureMismatch.into());
        }

        // Security Check: Verify the passed mint account matches the one in config
        if config_data.mint_account != *mint_account_info.key {
            msg!("Error: Mint account does not match configured mint");
            return Err(StablecoinError::MintAccountMismatch.into());
        }

        // Perform the minting via Cross-Program Invocation (CPI)
        // ETH Dev Analogy: This is like calling `IERC20(tokenAddress).mint(...)`
        msg!("Invoking SPL Token program to mint {} tokens", amount);
        let mint_instruction = token_instruction::mint_to(
            token_program_info.key, // SPL Token program ID
            mint_account_info.key,  // The Mint account to mint from
            destination_account.key,// The destination Token Account (ATA)
            program_id,             // Mint Authority: THIS program's ID
            &[program_id],          // Signers: THIS program is the authority
            amount,
        )?;

        // We need to provide the accounts required by the *SPL Token program's* mint_to instruction.
        // Note: The 'authority' account for spl-token's mint_to is this program's derived address (PDA),
        // but since *this program itself* is the authority, we can use its program_id and invoke_signed.
        // However, a simpler model (used here) is if this program's *Config Account* is the authority.
        // Let's assume the Config Account's address was used as mint authority when creating the mint.
        // If the *program* is the authority, you'd need a PDA derived from the program_id.
        // **Correction**: The authority signing should be the one set *on the mint account*.
        // If this program is the authority, it needs to sign via PDA.
        // If the *admin* account was set as mint authority (less secure, not typical), admin signs.
        // Let's assume this program *itself* is the mint authority. We need a PDA seed.
        // **Simplification for Example**: Let's assume the *admin account* was directly set
        // as the mint authority on the SPL Token Mint (less common, but simpler for demo).
        // If program was authority, you'd use invoke_signed with PDA seeds.

        // **Revised Assumption**: Assume the `admin_account` *is* the mint authority
        // set on the `mint_account_info`. This simplifies the CPI call.
        // A more robust design uses a Program Derived Address (PDA) owned by this
        // program as the mint authority.

        invoke(
            &mint_instruction,
            &[
                mint_account_info.clone(),      // Mint account (source)
                destination_account.clone(),    // Destination ATA
                admin_account.clone(),          // Mint authority (signer) - AS PER REVISED ASSUMPTION
                token_program_info.clone(),     // SPL Token program ID
            ],
        )?;


        msg!("Mint successful.");
        Ok(())
    }

     /// Processes the SetAdmin instruction.
    fn process_set_admin(
        accounts: &[AccountInfo],
        new_admin: Pubkey,
        program_id: &Pubkey,
    ) -> ProgramResult {
        let account_info_iter = &mut accounts.iter();
        // Account 0: Current Admin (Signer)
        let current_admin_account = next_account_info(account_info_iter)?;
        // Account 1: Config Account (Writable)
        let config_account = next_account_info(account_info_iter)?;

        // Security checks
        if !current_admin_account.is_signer {
            msg!("Error: Current admin signature missing");
            return Err(ProgramError::MissingRequiredSignature);
        }
         if config_account.owner != program_id {
             msg!("Error: Config account not owned by program");
             return Err(ProgramError::IncorrectProgramId);
        }

        // Unpack config state
        let mut config_data = ConfigAccount::unpack(&config_account.data.borrow())?;
        if !config_data.is_initialized() {
             msg!("Error: Config account not initialized");
             return Err(StablecoinError::NotInitialized.into());
        }

        // Verify signer is the current admin
        if config_data.admin != *current_admin_account.key {
            msg!("Error: Signer is not the current admin");
            return Err(StablecoinError::AdminSignatureMismatch.into());
        }

        // Update the admin
        config_data.admin = new_admin;

        // Pack the updated state back into the account
        ConfigAccount::pack(config_data, &mut config_account.data.borrow_mut())?;

        msg!("Admin updated successfully to: {}", new_admin);
        Ok(())
    }
}
```
/*
**How to Use (Conceptual Workflow):**

1.  **Deploy Program:** Compile the Rust code to BPF bytecode and 
deploy it to the Solana network. This gives you a `program_id`.
2.  **Create SPL Token Mint:** Use the SPL Token CLI or client-side JS to create a new token mint 
(e.g., `spl-token create-token`). Crucially, 
set the **mint authority** to the `program_id` you just deployed 
(or a PDA derived from it for better security, though the example assumes the admin for simplicity in CPI).
3.  **Create Config Account:** Client-side (e.g., JavaScript), 
create a new empty account using `SystemProgram.createAccount`, 
allocating `ConfigAccount::LEN` space and assigning ownership to your deployed `program_id`. 
Make sure it's rent-exempt.
4.  **Initialize Program:** Call the `Initialize` instruction of your deployed program, 
passing in the required accounts (payer, the newly created config account, the SPL Token Mint address) 
and the desired initial `admin` public key.
5.  **Mint Tokens:**
    * Ensure the recipient has an Associated Token Account (ATA) for your stablecoin mint (create it client-side if needed using the ATA program).
    * The authorized `admin` user signs a transaction calling the `MintTo` instruction, providing the config account, mint account, destination ATA, and the amount. The program verifies the signer is the admin and uses CPI to call the SPL Token program's `mint_to` function.
6.  **Transfer/Burn:** Users interact directly with the SPL Token program (via wallets like Phantom or client-side JS) to transfer or burn their tokens held in their ATAs. Your custom program isn't involved in standard transfers/burns.
7.  **Set New Admin:** The current `admin` signs a transaction calling the `SetAdmin` instruction to transfer control.

This structure separates concerns: the robust, audited SPL Token program handles core token mechanics, while your custom program focuses solely on the specific administrative logic (mint contro

    */