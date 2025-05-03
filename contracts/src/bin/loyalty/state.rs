use borsh::{BorshDeserialize, BorshSerialize};
use solana_program::{
    program_pack::{IsInitialized, Pack, Sealed},
    pubkey::Pubkey,
};

// Configuration state account for the loyalty program.
#[derive(BorshSerialize, BorshDeserialize, Debug, Clone, Default)]
pub struct ConfigAccount {
    /// Tracks if the account is initialized.
    pub is_initialized: bool,
    /// The public key authorized to award points and change the admin.
    /// ETH Dev Analogy: The 'owner' or 'admin' role address.
    pub admin: Pubkey,
    /// The public key of the SPL Token Mint account representing loyalty points.
    /// This program MUST be the mint_authority for this mint.
    pub loyalty_mint: Pubkey,
    // Add other config if needed, e.g., redemption treasury account
}

impl Sealed for ConfigAccount {}
impl IsInitialized for ConfigAccount {
    fn is_initialized(&self) -> bool {
        self.is_initialized
    }
}
impl Pack for ConfigAccount {
    // LEN: bool (1) + Pubkey (32) + Pubkey (32)
    const LEN: usize = 1 + 32 + 32;

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