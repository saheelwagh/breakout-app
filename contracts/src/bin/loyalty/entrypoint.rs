use solana_program::{
    account_info::AccountInfo, entrypoint, entrypoint::ProgramResult, pubkey::Pubkey,
};
use crate::{error::LoyaltyError, processor::Processor};

entrypoint!(process_instruction);

fn process_instruction(
    program_id: &Pubkey,
    accounts: &[AccountInfo],
    instruction_data: &[u8],
) -> ProgramResult {
    // Log entry and delegate processing (optional logging)
    // solana_program::msg!("Loyalty Program Entrypoint");
    if let Err(error) = Processor::process(program_id, accounts, instruction_data) {
        // error.print::<LoyaltyError>(); // Log detailed errors if needed
        return Err(error);
    }
    Ok(())
}