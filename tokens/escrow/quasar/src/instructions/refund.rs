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

#[inline(always)]
pub fn handle_withdraw_tokens_and_close_refund(accounts: &mut Refund, bumps: &RefundBumps) -> Result<(), ProgramError> {
    let seeds = accounts.escrow_seeds(bumps);

    accounts.token_program
        .transfer(
            &accounts.vault_ta_a,
            &accounts.maker_ta_a,
            &accounts.escrow,
            accounts.vault_ta_a.amount(),
        )
        .invoke_signed(&seeds)?;

    accounts.token_program
        .close_account(&accounts.vault_ta_a, &accounts.maker, &accounts.escrow)
        .invoke_signed(&seeds)?;
    Ok(())
}
