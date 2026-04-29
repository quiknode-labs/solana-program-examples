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

#[inline(always)]
pub fn handle_transfer_tokens(accounts: &mut Take) -> Result<(), ProgramError> {
    accounts.token_program
        .transfer(
            &accounts.taker_ta_b,
            &accounts.maker_ta_b,
            &accounts.taker,
            accounts.escrow.receive,
        )
        .invoke()
}

#[inline(always)]
pub fn handle_withdraw_tokens_and_close_take(accounts: &mut Take, bumps: &TakeBumps) -> Result<(), ProgramError> {
    let seeds = accounts.escrow_seeds(bumps);

    accounts.token_program
        .transfer(
            &accounts.vault_ta_a,
            &accounts.taker_ta_a,
            &accounts.escrow,
            accounts.vault_ta_a.amount(),
        )
        .invoke_signed(&seeds)?;

    accounts.token_program
        .close_account(&accounts.vault_ta_a, &accounts.taker, &accounts.escrow)
        .invoke_signed(&seeds)?;
    Ok(())
}
