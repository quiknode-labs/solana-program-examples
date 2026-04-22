use {
    crate::state::{Contributor, Fundraiser},
    quasar_lang::prelude::*,
    quasar_spl::{Token, TokenCpi},
};

#[derive(Accounts)]
pub struct Refund {
    #[account(mut)]
    pub contributor: Signer,
    pub maker: UncheckedAccount,
    #[account(
        mut,
        has_one = maker,
        seeds = Fundraiser::seeds(maker),
        bump = fundraiser.bump
    )]
    pub fundraiser: Account<Fundraiser>,
    #[account(mut)]
    pub contributor_account: Account<Contributor>,
    #[account(mut)]
    pub contributor_ta: Account<Token>,
    #[account(mut)]
    pub vault: Account<Token>,
    pub token_program: Program<Token>,
}

impl Refund {
    #[inline(always)]
    pub fn refund(&mut self, bumps: &RefundBumps) -> Result<(), ProgramError> {
        let refund_amount = self.contributor_account.amount;

        let seeds = self.fundraiser_seeds(bumps);

        // Transfer contributor's tokens back from vault
        self.token_program
            .transfer(&self.vault, &self.contributor_ta, &self.fundraiser, refund_amount)
            .invoke_signed(&seeds)?;

        // Update fundraiser state
        self.fundraiser.current_amount = self.fundraiser.current_amount
            .checked_sub(refund_amount)
            .ok_or(ProgramError::ArithmeticOverflow)?;

        // Zero out contributor amount
        self.contributor_account.set_inner(0);

        Ok(())
    }
}
