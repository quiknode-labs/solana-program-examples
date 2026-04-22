use {
    crate::state::Escrow,
    quasar_lang::prelude::*,
    quasar_spl::{Mint, Token, TokenCpi},
};

#[derive(Accounts)]
pub struct Refund {
    #[account(mut)]
    pub maker: Signer,
    #[account(
        mut,
        has_one = maker,
        close = maker,
        seeds = Escrow::seeds(maker),
        bump = escrow.bump
    )]
    pub escrow: Account<Escrow>,
    pub mint_a: Account<Mint>,
    #[account(mut, init_if_needed, payer = maker, token::mint = mint_a, token::authority = maker)]
    pub maker_ta_a: Account<Token>,
    #[account(mut)]
    pub vault_ta_a: Account<Token>,
    pub rent: Sysvar<Rent>,
    pub token_program: Program<Token>,
    pub system_program: Program<System>,
}

impl Refund {
    #[inline(always)]
    pub fn withdraw_tokens_and_close(&mut self, bumps: &RefundBumps) -> Result<(), ProgramError> {
        let seeds = self.escrow_seeds(bumps);

        self.token_program
            .transfer(
                &self.vault_ta_a,
                &self.maker_ta_a,
                &self.escrow,
                self.vault_ta_a.amount(),
            )
            .invoke_signed(&seeds)?;

        self.token_program
            .close_account(&self.vault_ta_a, &self.maker, &self.escrow)
            .invoke_signed(&seeds)?;
        Ok(())
    }
}
