use {
    crate::state::{Contributor, ContributorInner, Fundraiser},
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

#[inline(always)]
pub fn handle_refund(accounts: &mut Refund, bumps: &RefundBumps) -> Result<(), ProgramError> {
    let refund_amount = accounts.contributor_account.amount;

    let seeds = accounts.fundraiser_seeds(bumps);

    // Transfer contributor's tokens back from vault
    accounts.token_program
        .transfer(&accounts.vault, &accounts.contributor_ta, &accounts.fundraiser, refund_amount)
        .invoke_signed(&seeds)?;

    // Update fundraiser state
    accounts.fundraiser.current_amount = accounts.fundraiser.current_amount
        .checked_sub(refund_amount)
        .ok_or(ProgramError::ArithmeticOverflow)?;

    // Zero out contributor amount
    accounts.contributor_account.set_inner(ContributorInner { amount: 0 });

    Ok(())
}
