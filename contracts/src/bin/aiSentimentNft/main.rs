/*
This example outlines a Solana program for an NFT project where the NFT's state
 (which influences its appearance or attributes)
  changes based on AI analysis fed onto the chain via an oracle
  .The Workflow:Off-Chain AI & Oracle:
  An off-chain service monitors an external data source 
  (e.g., Twitter sentiment about a specific topic, project-related news).
  An AI model analyzes this data, producing a simple output (e.g., a sentiment score from 0-100).
  The oracle service signs this score along with a timestamp or nonce using its private key.
  The oracle posts this signed data to a specific data account on Solana (OracleDataAccount).
  On-Chain Solana Program:Manages configuration (like the trusted oracle's public key).
  Manages state accounts for each NFT (NftEvolutionAccount) storing things like 
  current_sentiment_score or 
  evolution_points.Provides an instruction (UpdateNftState) 
  that anyone can call (or perhaps only the NFT owner).
  UpdateNftState Instruction Logic:Reads the latest data and signature from the OracleDataAccount.
  Reads the trusted oracle public key from the program's ConfigAccount.
  Verifies the oracle's signature on the data using the trusted public key. 
  This is the crucial security step ensuring the data came from the expected oracle.
  If the signature is valid, it updates the specific NFT's NftEvolutionAccount based on the verified 
  AI sentiment score (e.g., increase evolution_points if sentiment is high).
  Emits an event (e.g., using msg!) signalling the state change.Off-Chain Interpretation:
  A front-end or metadata service listens for these on-chain state changes.When an NFT's state is updated,
   the off-chain service might update the NFT's image (if dynamically generated) 
   or its attributes in the JSON metadata file (stored on Arweave/IPFS) to reflect the new state 
   (e.g., change background color, add an accessory, update a "Mood" attribute).
   Key Solana Concepts
    Illustrated:Oracles: Integrating off-chain data/computation (AI results)
    .Cross-Program Invocation (CPI): Not directly used here, but often involved if interacting with other
     programs like Metaplex for metadata updates.S
     ignature Verification: Using on-chain functions to verify cryptographic signatures (e.g., Ed25519).
     State Management: Separating configuration, oracle data, and individual 
     NFT state into different accounts.Composability: Building on top of existing standards
      like SPL Tokens/NFTs (Metaplex).The following Rust code outlines the Solana program structure. 
      It focuses on the on-chain verification and state update logic.
  The actual AI model, oracle service, and off-chain metadata/image updates are external components.
*/

// === Cargo.toml Dependencies ===
// [dependencies]
// solana-program = "1.18.4"
// borsh = "1.4.0"
// thiserror = "1.0.58"
// spl-token = { version = "4.0.1", features = ["no-entrypoint"] } # If interacting with tokens/NFTs
// ed25519-dalek = { version = "2.1.1", default-features = false } # For Ed25519 signature verification
// sha3 = { version = "0.10.8", default-features = false } # For hashing message before verification

// === src/lib.rs, src/entrypoint.rs ===
// (Standard entrypoint setup as in previous examples)
/*
hose lines declare entrypoint, error, instruction, processor, and state as public modules within your Rust crate (likely the main lib.rs file).

Here's what that means:

Module Declaration: Each mod <name>; line tells the Rust compiler to look for the code corresponding to that module (e.g., in entrypoint.rs, error.rs, etc.).
Public Visibility (pub): The pub keyword makes the module itself visible and accessible to code outside of the file where these lines are written (e.g., potentially to other crates if this were a library, or to different parts of the same crate).
However, it's important to note that this doesn't automatically make everything inside those modules public. For code (structs, functions, enums, constants, etc.) defined within entrypoint.rs, processor.rs, etc., to be usable outside of their respective module files, those specific items must also be declared with the pub keyword.

So, these lines make the modules themselves accessible, but the accessibility of the code within them depends on whether those internal items are marked pub.
*/
pub mod entrypoint;
pub mod error;
pub mod instruction;
pub mod processor;
pub mod state;

// === src/state.rs ===
use borsh::{BorshDeserialize, BorshSerialize};
use solana_program::{
    program_pack::{IsInitialized, Pack, Sealed},
    pubkey::Pubkey,
};

/// Configuration account holding trusted oracle information.
/// Initialized once by the program admin.
#[derive(BorshSerialize, BorshDeserialize, Debug, Clone, Default)]
pub struct ConfigAccount {
    pub is_initialized: bool,
    /// The public key of the trusted off-chain oracle service.
    pub oracle_pubkey: Pubkey,
    // Could add other config like update frequency limits, etc.
}

impl Sealed for ConfigAccount {}
impl IsInitialized for ConfigAccount {
    fn is_initialized(&self) -> bool { self.is_initialized }
}
impl Pack for ConfigAccount {
    const LEN: usize = 1 + 32; // bool + Pubkey
    // Pack/Unpack implementations using Borsh (similar to stablecoin example)
     fn pack_into_slice(&self, dst: &mut [u8]) {
        let mut writer = std::io::Cursor::new(dst);
        self.serialize(&mut writer).unwrap();
    }
    fn unpack_from_slice(src: &[u8]) -> Result<Self, solana_program::program_error::ProgramError> {
        let mut reader = std::io::Cursor::new(src);
        ConfigAccount::deserialize(&mut reader)
            .map_err(|_| solana_program::program_error::ProgramError::InvalidAccountData)
    }
}


/// Account storing the latest data posted by the oracle.
/// This account is written to by the off-chain oracle service.
#[derive(BorshSerialize, BorshDeserialize, Debug, Clone, Default)]
pub struct OracleDataAccount {
    /// The latest AI-derived sentiment score (e.g., 0-100).
    pub sentiment_score: u64,
    /// Timestamp or nonce when the data was generated/posted.
    pub timestamp: i64,
    /// Signature from the oracle_pubkey over the score and timestamp.
    /// Stored as bytes (64 bytes for Ed25519).
    pub signature: [u8; 64],
}
// Note: This account is typically *not* marked as initialized or packed using Solana's Pack
// trait if it's only ever written to/read from directly by external services and this program.
// However, defining LEN is useful.
impl OracleDataAccount {
    pub const LEN: usize = 8 + 8 + 64; // u64 + i64 + signature bytes
}


/// Account storing the evolving state for a specific NFT.
/// There would be one such account per NFT in the collection.
#[derive(BorshSerialize, BorshDeserialize, Debug, Clone, Default)]
pub struct NftEvolutionAccount {
    pub is_initialized: bool,
    /// Link back to the NFT mint this state belongs to.
    pub nft_mint: Pubkey,
    /// The last sentiment score processed for this NFT.
    pub last_processed_sentiment: u64,
    /// Timestamp associated with the last processed score.
    pub last_processed_timestamp: i64,
    /// Points accumulated based on sentiment, driving evolution.
    pub evolution_points: u64,
    // Other state variables...
}

impl Sealed for NftEvolutionAccount {}
impl IsInitialized for NftEvolutionAccount {
    fn is_initialized(&self) -> bool { self.is_initialized }
}
impl Pack for NftEvolutionAccount {
    // Adjust LEN based on actual fields
    const LEN: usize = 1 + 32 + 8 + 8 + 8; // bool + Pubkey + u64 + i64 + u64
    // Pack/Unpack implementations using Borsh
     fn pack_into_slice(&self, dst: &mut [u8]) {
        let mut writer = std::io::Cursor::new(dst);
        self.serialize(&mut writer).unwrap();
    }
    fn unpack_from_slice(src: &[u8]) -> Result<Self, solana_program::program_error::ProgramError> {
        let mut reader = std::io::Cursor::new(src);
        NftEvolutionAccount::deserialize(&mut reader)
            .map_err(|_| solana_program::program_error::ProgramError::InvalidAccountData)
    }
}


// === src/instruction.rs ===
use borsh::{BorshDeserialize, BorshSerialize};
use solana_program::pubkey::Pubkey;

#[derive(BorshSerialize, BorshDeserialize, Debug, Clone)]
pub enum AiNftInstruction {
    /// Initializes the global configuration.
    /// Accounts:
    /// 0. `[signer]` Admin/Authority creating the config.
    /// 1. `[writable]` Config account to initialize.
    /// 2. `[]` Rent sysvar.
    /// 3. `[]` System program.
    InitializeConfig {
        oracle_pubkey: Pubkey,
    },

    /// Initializes the state account for a specific NFT.
    /// Accounts:
    /// 0. `[signer]` Payer for rent.
    /// 1. `[writable]` NftEvolutionAccount to initialize.
    /// 2. `[]` NFT Mint address this state account is for.
    /// 3. `[]` Rent sysvar.
    /// 4. `[]` System program.
    InitializeNftState,

    /// Updates the NFT's state based on the latest oracle data.
    /// Accounts:
    /// 0. `[signer]` User triggering the update (optional, could be anyone).
    /// 1. `[writable]` NftEvolutionAccount to update.
    /// 2. `[]` OracleDataAccount containing latest AI score and signature.
    /// 3. `[]` ConfigAccount containing the trusted oracle pubkey.
    UpdateNftState,
}


// === src/error.rs ===
use thiserror::Error;
use solana_program::program_error::ProgramError;

#[derive(Error, Debug, Copy, Clone, PartialEq)]
pub enum AiNftError {
    #[error("Invalid Instruction")]
    InvalidInstruction,
    #[error("Not Rent Exempt")]
    NotRentExempt,
    #[error("Already Initialized")]
    AlreadyInitialized,
    #[error("Account Not Initialized")]
    NotInitialized,
    #[error("Oracle signature verification failed")]
    OracleSignatureVerificationFailed,
    #[error("Invalid Oracle account owner")]
    InvalidOracleAccountOwner, // If checking owner
    #[error("Stale Oracle Data")]
    StaleOracleData,
    #[error("Data already processed")]
    DataAlreadyProcessed,
    #[error("Invalid Config account owner")]
    InvalidConfigAccountOwner,
    #[error("Invalid NFT state account owner")]
    InvalidNftStateAccountOwner,
}

impl From<AiNftError> for ProgramError {
    fn from(e: AiNftError) -> Self { ProgramError::Custom(e as u32) }
}


// === src/processor.rs ===
use solana_program::{
    account_info::{next_account_info, AccountInfo},
    entrypoint::ProgramResult,
    msg,
    program_error::ProgramError,
    program_pack::{IsInitialized, Pack},
    pubkey::Pubkey,
    sysvar::{rent::Rent, Sysvar},
};
// Import signature verification and hashing crates
use ed25519_dalek::{Signature, Verifier, VerifyingKey}; // Using 2.x version syntax
use sha3::{Digest, Keccak256}; // Example using Keccak256, adjust if needed

use crate::{
    error::AiNftError,
    instruction::AiNftInstruction,
    state::{ConfigAccount, OracleDataAccount, NftEvolutionAccount},
};


pub struct Processor;
impl Processor {
    pub fn process(
        program_id: &Pubkey,
        accounts: &[AccountInfo],
        instruction_data: &[u8],
    ) -> ProgramResult {
        let instruction = AiNftInstruction::try_from_slice(instruction_data)
            .map_err(|_| ProgramError::InvalidInstructionData)?;

        match instruction {
            AiNftInstruction::InitializeConfig { oracle_pubkey } => {
                 msg!("Instruction: InitializeConfig");
                 Self::process_initialize_config(accounts, oracle_pubkey, program_id)
            }
            AiNftInstruction::InitializeNftState => {
                 msg!("Instruction: InitializeNftState");
                 Self::process_initialize_nft_state(accounts, program_id)
            }
            AiNftInstruction::UpdateNftState => {
                 msg!("Instruction: UpdateNftState");
                 Self::process_update_nft_state(accounts, program_id)
            }
        }
    }

    // --- Initialize Config Implementation ---
    fn process_initialize_config(
        accounts: &[AccountInfo],
        oracle_pubkey: Pubkey,
        program_id: &Pubkey,
    ) -> ProgramResult {
        let account_info_iter = &mut accounts.iter();
        let _admin_account = next_account_info(account_info_iter)?; // Signer
        let config_account = next_account_info(account_info_iter)?; // Writable
        let rent_sysvar_account = next_account_info(account_info_iter)?; // Rent
        let _system_program = next_account_info(account_info_iter)?; // System

        // Check ownership, rent-exemption, initialization status (similar to stablecoin)
         if config_account.owner != program_id {
             return Err(AiNftError::InvalidConfigAccountOwner.into());
         }
         let rent = Rent::from_account_info(rent_sysvar_account)?;
         if !rent.is_exempt(config_account.lamports(), config_account.data_len()) {
             return Err(AiNftError::NotRentExempt.into());
         }
         let mut config_data = ConfigAccount::unpack_unchecked(&config_account.data.borrow())?;
         if config_data.is_initialized() {
             return Err(AiNftError::AlreadyInitialized.into());
         }

        // Initialize
        config_data.is_initialized = true;
        config_data.oracle_pubkey = oracle_pubkey;
        ConfigAccount::pack(config_data, &mut config_account.data.borrow_mut())?;
        msg!("Config initialized with Oracle Pubkey: {}", oracle_pubkey);
        Ok(())
    }

     // --- Initialize NFT State Implementation ---
    fn process_initialize_nft_state(
        accounts: &[AccountInfo],
        program_id: &Pubkey,
    ) -> ProgramResult {
        let account_info_iter = &mut accounts.iter();
        let _payer_account = next_account_info(account_info_iter)?; // Signer
        let nft_state_account = next_account_info(account_info_iter)?; // Writable
        let nft_mint_account = next_account_info(account_info_iter)?; // Readonly
        let rent_sysvar_account = next_account_info(account_info_iter)?; // Rent
        let _system_program = next_account_info(account_info_iter)?; // System

        // Check ownership, rent-exemption, initialization status
         if nft_state_account.owner != program_id {
             return Err(AiNftError::InvalidNftStateAccountOwner.into());
         }
        let rent = Rent::from_account_info(rent_sysvar_account)?;
        if !rent.is_exempt(nft_state_account.lamports(), nft_state_account.data_len()) {
            return Err(AiNftError::NotRentExempt.into());
        }
        let mut nft_state_data = NftEvolutionAccount::unpack_unchecked(&nft_state_account.data.borrow())?;
        if nft_state_data.is_initialized() {
            return Err(AiNftError::AlreadyInitialized.into());
        }

        // Initialize
        nft_state_data.is_initialized = true;
        nft_state_data.nft_mint = *nft_mint_account.key;
        nft_state_data.last_processed_sentiment = 0; // Initial values
        nft_state_data.last_processed_timestamp = 0;
        nft_state_data.evolution_points = 0;
        NftEvolutionAccount::pack(nft_state_data, &mut nft_state_account.data.borrow_mut())?;
        msg!("NFT state initialized for mint: {}", nft_mint_account.key);
        Ok(())
    }


    // --- Update NFT State Implementation ---
    fn process_update_nft_state(
        accounts: &[AccountInfo],
        program_id: &Pubkey,
    ) -> ProgramResult {
        let account_info_iter = &mut accounts.iter();
        let _updater_account = next_account_info(account_info_iter)?; // Signer (optional usage)
        let nft_state_account = next_account_info(account_info_iter)?; // Writable
        let oracle_data_account = next_account_info(account_info_iter)?; // Readonly
        let config_account = next_account_info(account_info_iter)?; // Readonly

        // --- Load Accounts & Basic Checks ---
        if nft_state_account.owner != program_id {
             return Err(AiNftError::InvalidNftStateAccountOwner.into());
        }
         if config_account.owner != program_id {
             return Err(AiNftError::InvalidConfigAccountOwner.into());
        }
        // Optional: Check oracle_data_account owner if it's managed by a specific program/key
        // if oracle_data_account.owner != &expected_oracle_program_or_key { ... }

        let config_data = ConfigAccount::unpack(&config_account.data.borrow())?;
        if !config_data.is_initialized() {
             msg!("Error: Config account not initialized");
             return Err(AiNftError::NotInitialized.into());
        }
        let mut nft_state_data = NftEvolutionAccount::unpack(&nft_state_account.data.borrow())?;
         if !nft_state_data.is_initialized() {
             msg!("Error: NFT state account not initialized");
             return Err(AiNftError::NotInitialized.into());
        }

        // --- Deserialize Oracle Data ---
        // Use appropriate deserialization if OracleDataAccount uses Pack/Borsh
        // Here, we assume direct byte access for simplicity if it's just raw data written by oracle.
        let oracle_data_bytes = oracle_data_account.data.borrow();
        // Ensure data length is correct before slicing
        if oracle_data_bytes.len() < OracleDataAccount::LEN {
             return Err(ProgramError::InvalidAccountData);
        }
        // Manually slice and deserialize (example assuming layout in OracleDataAccount)
        let score_bytes: [u8; 8] = oracle_data_bytes[0..8].try_into().unwrap();
        let timestamp_bytes: [u8; 8] = oracle_data_bytes[8..16].try_into().unwrap();
        let signature_bytes: [u8; 64] = oracle_data_bytes[16..80].try_into().unwrap();

        let oracle_sentiment_score = u64::from_le_bytes(score_bytes);
        let oracle_timestamp = i64::from_le_bytes(timestamp_bytes);
        let oracle_signature = Signature::from_bytes(&signature_bytes)
            .map_err(|_| AiNftError::OracleSignatureVerificationFailed)?; // Handle potential error

        // --- Check for Stale/Replay ---
        if oracle_timestamp <= nft_state_data.last_processed_timestamp {
            msg!("Error: Oracle data timestamp is not newer than last processed");
            return Err(AiNftError::DataAlreadyProcessed.into());
        }

        // --- Verify Oracle Signature ---
        msg!("Verifying oracle signature...");

        // 1. Reconstruct the message that was signed by the oracle
        // IMPORTANT: This must EXACTLY match how the oracle constructed the message off-chain.
        // Example: Concatenate score and timestamp bytes. Use a specific hash if the oracle did.
        let mut message_bytes = Vec::with_capacity(16); // 8 bytes for score + 8 for timestamp
        message_bytes.extend_from_slice(&oracle_sentiment_score.to_le_bytes());
        message_bytes.extend_from_slice(&oracle_timestamp.to_le_bytes());

        // Optional: Hash the message if the oracle signed the hash
        // let mut hasher = Keccak256::new(); // Or Sha256, etc.
        // hasher.update(&message_bytes);
        // let message_hash = hasher.finalize();
        // let message_to_verify = message_hash.as_slice();

        // Use raw message bytes if oracle signed the raw data directly
        let message_to_verify = message_bytes.as_slice();


        // 2. Get the oracle's public key from config
        let oracle_verifying_key = VerifyingKey::from_bytes(&config_data.oracle_pubkey.to_bytes())
            .map_err(|_| ProgramError::InvalidAccountData)?; // Handle potential error if key is invalid

        // 3. Perform Ed25519 verification
        oracle_verifying_key.verify_strict(message_to_verify, &oracle_signature)
            .map_err(|e| {
                msg!("Signature verification failed: {:?}", e);
                AiNftError::OracleSignatureVerificationFailed
            })?;

        msg!("Oracle signature verified successfully!");

        // --- Update NFT State Based on Verified Data ---
        nft_state_data.last_processed_sentiment = oracle_sentiment_score;
        nft_state_data.last_processed_timestamp = oracle_timestamp;

        // Example logic: Add points based on sentiment score
        if oracle_sentiment_score > 75 {
            nft_state_data.evolution_points += 10;
        } else if oracle_sentiment_score > 50 {
             nft_state_data.evolution_points += 5;
        } else if oracle_sentiment_score < 25 {
             // Maybe decrease points or trigger a negative effect?
             nft_state_data.evolution_points = nft_state_data.evolution_points.saturating_sub(2);
        }
        // Add more complex logic here based on score, points, etc.

        msg!(
            "NFT state updated for mint {}: Score={}, Timestamp={}, New Points={}",
            nft_state_data.nft_mint,
            nft_state_data.last_processed_sentiment,
            nft_state_data.last_processed_timestamp,
            nft_state_data.evolution_points
        );

        // --- Save Updated NFT State ---
        NftEvolutionAccount::pack(nft_state_data, &mut nft_state_account.data.borrow_mut())?;

        Ok(())
    }
}
/*

**Explanation and Considerations:**

1.  **Oracle Trust:** This entire system relies on trusting the oracle service (identified by `oracle_pubkey`) to run the AI correctly and post accurate, timely data.
2.  **Signature Verification:** The `process_update_nft_state` function performs the critical Ed25519 signature check. It reconstructs the exact message the oracle signed (important!) and verifies it against the signature and the trusted public key.
3.  **State Updates:** The on-chain program only stores minimal state derived from the AI (e.g., `evolution_points`). The complex AI logic is off-chain.
4.  **Off-Chain Components:** Remember this requires significant off-chain infrastructure: the AI model, the service to run it, the oracle service to sign and post data, and likely a service to update NFT visuals/metadata based on on-chain state changes.
5.  **Gas/Compute:** Signature verification consumes compute units. Keep the signed message reasonably small and the verification logic efficient.
6.  **Data Freshness/Replay:** The timestamp check (`oracle_timestamp <= nft_state_data.last_processed_timestamp`) prevents processing old data or the same data multiple times.
7.  **Error Handling:** Robust error handling is crucial, especially around signature verification and account deserialization.

This example provides a framework for integrating AI results into a Solana program in a secure (via signature verification) and feasible way, enabling interesting dynamics like AI-driven NFT evoluti
*/