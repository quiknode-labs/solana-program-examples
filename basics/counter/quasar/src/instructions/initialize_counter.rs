use {
    crate::state::Counter,
    quasar_lang::prelude::*,
};

/// Accounts for creating a new counter.
/// The counter is derived as a PDA from ["counter", payer] seeds.
#[derive(Accounts)]
pub struct InitializeCounter {
    #[account(mut)]
    pub payer: Signer,
    #[account(mut, init, payer = payer, seeds = Counter::seeds(payer), bump)]
    pub counter: Account<Counter>,
    pub system_program: Program<System>,
}

impl InitializeCounter {
    #[inline(always)]
    pub fn initialize_counter(&mut self) -> Result<(), ProgramError> {
        self.counter.set_inner(0u64);
        Ok(())
    }
}
