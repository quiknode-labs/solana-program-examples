use quasar_lang::prelude::*;

/// Accounts for creating a system account sized for address data.
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
    pub fn create_system_account(&mut self, name: &str, address: &str) -> Result<(), ProgramError> {
        // Calculate space needed for the serialised AddressData:
        // borsh-style: 4-byte length prefix + bytes for each String field.
        let space = 4 + name.len() + 4 + address.len();

        log("Program invoked. Creating a system account...");

        // The owner of the new account is the system program.
        let system_program_address = Address::default();

        // Create the account with the computed space.
        // create_account_with_minimum_balance automatically fetches Rent
        // sysvar and calculates the minimum rent-exempt lamports.
        self.system_program
            .create_account_with_minimum_balance(
                &self.payer,
                &self.new_account,
                space as u64,
                &system_program_address,
                None, // fetch Rent sysvar automatically
            )?
            .invoke()?;

        log("Account created successfully.");
        Ok(())
    }
}
