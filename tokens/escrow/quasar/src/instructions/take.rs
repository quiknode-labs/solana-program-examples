use {
    crate::state::Escrow,
    quasar_lang::prelude::*,
    quasar_spl::{Mint, Token, TokenCpi},
};

#[derive(Accounts)]
pub struct Take {
    #[account(mut)]
    pub taker: Signer,
    #[account(
        mut,
        has_one = maker,
        has_one = maker_ta_b,
        constraint = escrow.receive > 0,
        close = taker,
        seeds = Escrow::seeds(maker),
        bump = escrow.bump
    )]
    pub escrow: Account<Escrow>,
    #[account(mut)]
    pub maker: UncheckedAccount,
    pub mint_a: Account<Mint>,
    pub mint_b: Account<Mint>,
    #[account(mut, init_if_needed, payer = taker, token::mint = mint_a, token::authority = taker)]
    pub taker_ta_a: Account<Token>,
    #[account(mut)]
    pub taker_ta_b: Account<Token>,
    #[account(mut, init_if_needed, payer = taker, token::mint = mint_b, token::authority = maker)]
    pub maker_ta_b: Account<Token>,
    #[account(mut)]
    pub vault_ta_a: Account<Token>,
    pub rent: Sysvar<Rent>,
    pub token_program: Program<Token>,
    pub system_program: Program<System>,
}

impl Take {
    #[inline(always)]
    pub fn transfer_tokens(&mut self) -> Result<(), ProgramError> {
        self.token_program
            .transfer(
                &self.taker_ta_b,
                &self.maker_ta_b,
                &self.taker,
                self.escrow.receive,
            )
            .invoke()
    }

    #[inline(always)]
    pub fn withdraw_tokens_and_close(&mut self, bumps: &TakeBumps) -> Result<(), ProgramError> {
        let seeds = self.escrow_seeds(bumps);

        self.token_program
            .transfer(
                &self.vault_ta_a,
                &self.taker_ta_a,
                &self.escrow,
                self.vault_ta_a.amount(),
            )
            .invoke_signed(&seeds)?;

        self.token_program
            .close_account(&self.vault_ta_a, &self.taker, &self.escrow)
            .invoke_signed(&seeds)?;
        Ok(())
    }
}
