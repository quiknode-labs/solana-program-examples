use quasar_lang::prelude::*;

/// Accounts for creating a new system-owned account.
/// Both payer and new_account must sign the transaction.
#[derive(Accounts)]
pub struct CreateSystemAccount {
    #[account(mut)]
    pub payer: Signer,
    #[account(mut)]
    pub new_account: Signer,
    pub system_program: Program<System>,
}

impl CreateSystemAccount {
    #[inline(always)]
    pub fn create_system_account(&mut self) -> Result<(), ProgramError> {
        // Create a zero-data account owned by the system program,
        // funded with the minimum rent-exempt balance.
        let system_program_address = Address::default();
        self.system_program
            .create_account_with_minimum_balance(
                &self.payer,
                &self.new_account,
                0, // space: zero bytes of data
                &system_program_address,
                None, // fetch Rent sysvar automatically
            )?
            .invoke()
    }
}
