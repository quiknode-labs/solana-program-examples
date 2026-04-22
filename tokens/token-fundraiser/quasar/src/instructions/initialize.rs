use {
    crate::state::Fundraiser,
    quasar_lang::prelude::*,
    quasar_spl::{Mint, Token},
};

#[derive(Accounts)]
pub struct Initialize {
    #[account(mut)]
    pub maker: Signer,
    pub mint_to_raise: Account<Mint>,
    #[account(mut, init, payer = maker, seeds = Fundraiser::seeds(maker), bump)]
    pub fundraiser: Account<Fundraiser>,
    #[account(mut, init_if_needed, payer = maker, token::mint = mint_to_raise, token::authority = fundraiser)]
    pub vault: Account<Token>,
    pub rent: Sysvar<Rent>,
    pub token_program: Program<Token>,
    pub system_program: Program<System>,
}

impl Initialize {
    #[inline(always)]
    pub fn initialize(
        &mut self,
        amount_to_raise: u64,
        duration: u16,
        bump: u8,
    ) -> Result<(), ProgramError> {
        // Validate minimum raise amount
        require!(amount_to_raise > 0, ProgramError::InvalidArgument);

        self.fundraiser.set_inner(
            *self.maker.address(),
            *self.mint_to_raise.address(),
            amount_to_raise,
            0,  // current_amount starts at 0
            0,  // time_started — would be Clock::get() onchain
            duration,
            bump,
        );
        Ok(())
    }
}
