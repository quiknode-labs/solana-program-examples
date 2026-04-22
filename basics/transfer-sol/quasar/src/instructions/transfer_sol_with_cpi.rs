use quasar_lang::prelude::*;

/// Accounts for transferring SOL via system program CPI.
#[derive(Accounts)]
pub struct TransferSolWithCpi {
    #[account(mut)]
    pub payer: Signer,
    #[account(mut)]
    pub recipient: UncheckedAccount,
    pub system_program: Program<System>,
}

impl TransferSolWithCpi {
    #[inline(always)]
    pub fn transfer_sol_with_cpi(&mut self, amount: u64) -> Result<(), ProgramError> {
        self.system_program
            .transfer(&self.payer, &self.recipient, amount)
            .invoke()
    }
}
