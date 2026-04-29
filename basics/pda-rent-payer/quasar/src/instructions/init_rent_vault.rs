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

#[inline(always)]
pub fn handle_init_rent_vault(accounts: &mut InitRentVault, fund_lamports: u64) -> Result<(), ProgramError> {
    accounts.system_program
        .transfer(&accounts.payer, &accounts.rent_vault, fund_lamports)
        .invoke()
}
