use borsh::{BorshDeserialize, BorshSerialize};
use solana_program::pubkey::Pubkey;

/// Defines the instructions for the loyalty program.
/// ETH Dev Analogy: Public functions in a Solidity contract.
#[derive(BorshSerialize, BorshDeserialize, Debug, Clone)]
pub enum LoyaltyInstruction {
    /// Initializes the loyalty program configuration. Must be called once.
    ///
    /// Accounts expected:
    /// 0. `[signer]` Payer/Admin initializing the program.
    /// 1. `[writable]` Config account (needs to be created via SystemProgram first).
    /// 2. `[]` Loyalty Point SPL Token Mint address.
    /// 3. `[]` Rent sysvar.
    /// 4. `[]` System program.
    Initialize {
        /// The initial admin address.
        admin: Pubkey,
    },

    /// Awards loyalty points (mints tokens) to a user's token account.
    /// Only callable by the current admin.
    ///
    /// Accounts expected:
    /// 0. `[signer]` Current Admin account (must match `config_account.admin`).
    /// 1. `[]` Config account (holds admin and mint info).
    /// 2. `[writable]` Loyalty Point SPL Token Mint account (the mint address stored in config).
    /// 3. `[writable]` Destination User SPL Token Account (ATA of the recipient). Must exist.
    /// 4. `[]` SPL Token Program ID.
    /// 5. `[]` This Program's ID (as Mint Authority) - or PDA if using PDA authority.
    AwardPoints {
        /// Amount of loyalty points (smallest unit) to award.
        amount: u64,
    },

    /// Redeems (burns) loyalty points from a user's token account.
    /// Callable by the user who owns the points.
    ///
    /// Accounts expected:
    /// 0. `[signer]` User redeeming points (owner of the source token account).
    /// 1. `[writable]` User's Source SPL Token Account (ATA holding the points).
    /// 2. `[writable]` Loyalty Point SPL Token Mint account.
    /// 3. `[]` SPL Token Program ID.
    RedeemPoints {
        /// Amount of loyalty points (smallest unit) to redeem.
        amount: u64,
    },

    /// Sets a new admin for the loyalty program.
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
