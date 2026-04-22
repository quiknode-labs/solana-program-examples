use quasar_lang::prelude::*;

/// Accounts for funding the rent vault PDA.
/// Transfers lamports from the payer to the vault via system program CPI.
/// When lamports are sent to a new address, the system program creates
/// a system-owned account automatically.
#[derive(Accounts)]
pub struct InitRentVault {
    #[account(mut)]
    pub payer: Signer,
    #[account(mut, seeds = [b"rent_vault"], bump)]
    pub rent_vault: UncheckedAccount,
    pub system_program: Program<System>,
}

impl InitRentVault {
    #[inline(always)]
    pub fn init_rent_vault(&mut self, fund_lamports: u64) -> Result<(), ProgramError> {
        self.system_program
            .transfer(&self.payer, &self.rent_vault, fund_lamports)
            .invoke()
    }
}
