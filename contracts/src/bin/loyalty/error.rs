// === src/error.rs ===
use thiserror::Error;
use solana_program::program_error::ProgramError;

#[derive(Error, Debug, Copy, Clone, PartialEq)]
pub enum LoyaltyError {
    #[error("Invalid Instruction Data")]
    InvalidInstruction,
    #[error("Account Not Rent Exempt")]
    NotRentExempt,
    #[error("Account Already Initialized")]
    AlreadyInitialized,
    #[error("Account Not Initialized")]
    NotInitialized,
    #[error("Admin signature mismatch or missing")]
    AdminSignatureMismatch,
    #[error("Mint account mismatch")]
    MintAccountMismatch,
    #[error("Invalid Config account owner")]
    InvalidConfigAccountOwner,
    #[error("Numerical overflow error")]
    NumericalOverflow,
    #[error("Owner mismatch for token account")]
    OwnerMismatch, // For redeem check
}

impl From<LoyaltyError> for ProgramError {
    fn from(e: LoyaltyError) -> Self {
        ProgramError::Custom(e as u32)
    }
}