use {
    crate::state::PageVisits,
    quasar_lang::prelude::*,
};

/// Accounts for creating a new page visits counter.
/// The counter is derived as a PDA from ["page_visits", payer] seeds.
#[derive(Accounts)]
pub struct CreatePageVisits {
    #[account(mut)]
    pub payer: Signer,
    #[account(mut, init, payer = payer, seeds = PageVisits::seeds(payer), bump)]
    pub page_visits: Account<PageVisits>,
    pub system_program: Program<System>,
}

impl CreatePageVisits {
    #[inline(always)]
    pub fn create_page_visits(&mut self) -> Result<(), ProgramError> {
        self.page_visits.set_inner(0u64);
        Ok(())
    }
}
