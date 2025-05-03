use solana_program::{
    account_info::{next_account_info, AccountInfo},
    entrypoint::ProgramResult,
    msg,
    program::{invoke, invoke_signed}, // For CPI
    program_error::ProgramError,
    program_pack::{IsInitialized, Pack},
    pubkey::Pubkey,
    sysvar::{rent::Rent, Sysvar},
};
use spl_token::{
    instruction as token_instruction,
    state::Account as TokenAccount, // To check token account owner
};
use crate::{
    error::LoyaltyError,
    instruction::LoyaltyInstruction,
    state::ConfigAccount,
};

pub struct Processor;
impl Processor {
    pub fn process(
        program_id: &Pubkey,
        accounts: &[AccountInfo],
        instruction_data: &[u8],
    ) -> ProgramResult {
        let instruction = LoyaltyInstruction::try_from_slice(instruction_data)
            .map_err(|_| ProgramError::InvalidInstructionData)?;

        match instruction {
            LoyaltyInstruction::Initialize { admin } => {
                msg!("Instruction: Initialize");
                Self::process_initialize(accounts, admin, program_id)
            }
            LoyaltyInstruction::AwardPoints { amount } => {
                msg!("Instruction: AwardPoints");
                Self::process_award_points(accounts, amount, program_id)
            }
            LoyaltyInstruction::RedeemPoints { amount } => {
                msg!("Instruction: RedeemPoints");
                Self::process_redeem_points(accounts, amount, program_id)
            }
             LoyaltyInstruction::SetAdmin { new_admin } => {
                msg!("Instruction: SetAdmin");
                Self::process_set_admin(accounts, new_admin, program_id)
            }
        }
    }

    /// Processes Initialize instruction.
    fn process_initialize(
        accounts: &[AccountInfo],
        admin: Pubkey,
        program_id: &Pubkey,
    ) -> ProgramResult {
        let account_info_iter = &mut accounts.iter();
        let initializer_account = next_account_info(account_info_iter)?; // Signer
        let config_account = next_account_info(account_info_iter)?;      // Writable
        let loyalty_mint_account = next_account_info(account_info_iter)?; // Readonly
        let rent_sysvar_account = next_account_info(account_info_iter)?; // Rent
        let _system_program = next_account_info(account_info_iter)?;     // System

        if !initializer_account.is_signer {
             msg!("Initializer signature missing");
             return Err(ProgramError::MissingRequiredSignature);
        }

        // Check ownership, rent-exemption, initialization status
        if config_account.owner != program_id {
             msg!("Error: Config account not owned by program");
             return Err(LoyaltyError::InvalidConfigAccountOwner.into());
        }
        let rent = Rent::from_account_info(rent_sysvar_account)?;
        if !rent.is_exempt(config_account.lamports(), config_account.data_len()) {
             msg!("Error: Config account not rent exempt");
             return Err(LoyaltyError::NotRentExempt.into());
        }
        let mut config_data = ConfigAccount::unpack_unchecked(&config_account.data.borrow())?;
        if config_data.is_initialized() {
             msg!("Error: Config account already initialized");
             return Err(LoyaltyError::AlreadyInitialized.into());
        }

        // Initialize state
        config_data.is_initialized = true;
        config_data.admin = admin;
        config_data.loyalty_mint = *loyalty_mint_account.key;

        ConfigAccount::pack(config_data, &mut config_account.data.borrow_mut())?;
        msg!("Loyalty Config initialized. Admin: {}, Mint: {}", admin, loyalty_mint_account.key);
        Ok(())
    }

    /// Processes AwardPoints instruction.
    fn process_award_points(
        accounts: &[AccountInfo],
        amount: u64,
        program_id: &Pubkey,
    ) -> ProgramResult {
        let account_info_iter = &mut accounts.iter();
        let admin_account = next_account_info(account_info_iter)?;         // Signer
        let config_account = next_account_info(account_info_iter)?;         // Readonly
        let loyalty_mint_account = next_account_info(account_info_iter)?;  // Writable (SPL Token requires)
        let destination_token_account = next_account_info(account_info_iter)?; // Writable
        let token_program_account = next_account_info(account_info_iter)?; // Readonly (SPL Token Program ID)
        // Account 5: Mint Authority (this program_id or its PDA) is implicitly derived or passed if needed for invoke_signed

        // --- Validation ---
        if !admin_account.is_signer {
            msg!("Error: Admin signature missing");
            return Err(ProgramError::MissingRequiredSignature);
        }
        if config_account.owner != program_id {
             msg!("Error: Config account not owned by program");
             return Err(LoyaltyError::InvalidConfigAccountOwner.into());
        }

        let config_data = ConfigAccount::unpack(&config_account.data.borrow())?;
        if !config_data.is_initialized() {
             msg!("Error: Config account not initialized");
             return Err(LoyaltyError::NotInitialized.into());
        }

        // Check if signer is the admin
        if config_data.admin != *admin_account.key {
            msg!("Error: Signer is not the configured admin");
            return Err(LoyaltyError::AdminSignatureMismatch.into());
        }

        // Check if the provided mint matches the one in config
        if config_data.loyalty_mint != *loyalty_mint_account.key {
            msg!("Error: Mint account does not match configured mint");
            return Err(LoyaltyError::MintAccountMismatch.into());
        }

        // --- CPI to SPL Token Program ---
        msg!("Awarding {} loyalty points to {}", amount, destination_token_account.key);

        // ** IMPORTANT: Mint Authority Assumption **
        // This assumes this program's address (`program_id`) was set as the
        // mint authority when the `loyalty_mint_account` was created.
        // If a PDA is the authority, use `invoke_signed` with PDA seeds.
        let mint_cpi_instruction = token_instruction::mint_to(
            token_program_account.key,    // SPL Token program ID
            loyalty_mint_account.key,     // The Mint to mint from
            destination_token_account.key,// Destination user ATA
            program_id,                   // Mint Authority (this program's ID)
            &[program_id],                // Signer seeds (empty if program_id is authority)
            amount,
        )?;

        invoke(
            &mint_cpi_instruction,
            &[
                loyalty_mint_account.clone(),       // Mint account
                destination_token_account.clone(),  // Destination ATA
                token_program_account.clone(),      // SPL Token program ID
                // Authority account info - If program_id is authority, it doesn't need to be passed
                // explicitly here as it's derived by invoke/invoke_signed.
                // If using PDA, pass the PDA account info here.
                // If the *admin* was authority (less secure), pass admin_account.clone().
                // Let's assume program_id is authority for simplicity:
                // We need an AccountInfo for program_id if invoke requires it as authority
                // However, typically the authority is implicitly handled when it's the calling program.
                // Let's refine this - the authority needs to be provided.
                // We need an AccountInfo representing this program itself as the authority.
                // This is tricky. Let's assume the *admin* is the authority for this simpler example.
                // ** REVISED ASSUMPTION: Admin account is the mint authority **
                admin_account.clone(),             // Mint Authority (admin - Revised Assumption)
            ],
        )?;


        msg!("Points awarded successfully.");
        Ok(())
    }

     /// Processes RedeemPoints instruction.
    fn process_redeem_points(
        accounts: &[AccountInfo],
        amount: u64,
        program_id: &Pubkey, // program_id not strictly needed here but good practice
    ) -> ProgramResult {
         let account_info_iter = &mut accounts.iter();
         let user_account = next_account_info(account_info_iter)?;           // Signer (owner of source_token_account)
         let source_token_account = next_account_info(account_info_iter)?;   // Writable (User's ATA)
         let loyalty_mint_account = next_account_info(account_info_iter)?;   // Writable (SPL Token requires)
         let token_program_account = next_account_info(account_info_iter)?; // Readonly (SPL Token Program ID)

         // --- Validation ---
         if !user_account.is_signer {
             msg!("Error: User signature missing for redemption");
             return Err(ProgramError::MissingRequiredSignature);
         }

         // Check that the user_account (signer) is the owner of the source_token_account
         let token_account_data = TokenAccount::unpack(&source_token_account.data.borrow())?;
         if token_account_data.owner != *user_account.key {
             msg!("Error: Signer is not the owner of the source token account");
             return Err(LoyaltyError::OwnerMismatch.into());
         }

         // Check that the token account is for the correct mint
         if token_account_data.mint != *loyalty_mint_account.key {
              msg!("Error: Source token account is for the wrong mint");
              return Err(LoyaltyError::MintAccountMismatch.into()); // Re-use error or add specific one
         }


         // --- CPI to SPL Token Program to Burn ---
         msg!("Redeeming (burning) {} loyalty points from {}", amount, source_token_account.key);

         let burn_cpi_instruction = token_instruction::burn(
             token_program_account.key,    // SPL Token program ID
             source_token_account.key,     // Account to burn from
             loyalty_mint_account.key,     // Mint of the token
             user_account.key,             // Owner of the source account (authority)
             &[user_account.key],          // Signers (owner must sign)
             amount,
         )?;

         invoke(
             &burn_cpi_instruction,
             &[
                 source_token_account.clone(),   // Source ATA
                 loyalty_mint_account.clone(),   // Mint account
                 user_account.clone(),           // Authority (owner) signing
                 token_program_account.clone(),  // SPL Token program ID
             ],
         )?;

         msg!("Points redeemed successfully.");
         Ok(())
    }

    /// Processes SetAdmin instruction.
     fn process_set_admin(
        accounts: &[AccountInfo],
        new_admin: Pubkey,
        program_id: &Pubkey,
    ) -> ProgramResult {
        let account_info_iter = &mut accounts.iter();
        let current_admin_account = next_account_info(account_info_iter)?; // Signer
        let config_account = next_account_info(account_info_iter)?;      // Writable

        if !current_admin_account.is_signer {
            msg!("Error: Current admin signature missing");
            return Err(ProgramError::MissingRequiredSignature);
        }
         if config_account.owner != program_id {
             msg!("Error: Config account not owned by program");
             return Err(LoyaltyError::InvalidConfigAccountOwner.into());
        }

        let mut config_data = ConfigAccount::unpack(&config_account.data.borrow())?;
        if !config_data.is_initialized() {
             msg!("Error: Config account not initialized");
             return Err(LoyaltyError::NotInitialized.into());
        }

        // Verify signer is the current admin
        if config_data.admin != *current_admin_account.key {
            msg!("Error: Signer is not the current admin");
            return Err(LoyaltyError::AdminSignatureMismatch.into());
        }

        // Update the admin
        config_data.admin = new_admin;
        ConfigAccount::pack(config_data, &mut config_account.data.borrow_mut())?;

        msg!("Loyalty program admin updated successfully to: {}", new_admin);
        Ok(())
    }
}