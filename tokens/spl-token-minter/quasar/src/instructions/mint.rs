use quasar_lang::prelude::*;
use quasar_spl::{Mint, Token, TokenCpi};

/// Accounts for minting tokens to a recipient's token account.
#[derive(Accounts)]
pub struct MintToken {
    #[account(mut)]
    pub mint_authority: Signer,
    pub recipient: UncheckedAccount,
    #[account(mut)]
    pub mint_account: Account<Mint>,
    #[account(mut, init_if_needed, payer = mint_authority, token::mint = mint_account, token::authority = recipient)]
    pub associated_token_account: Account<Token>,
    pub token_program: Program<Token>,
    pub system_program: Program<System>,
}

#[inline(always)]
pub fn handle_mint_token(accounts: &mut MintToken, amount: u64) -> Result<(), ProgramError> {
    log("Minting tokens to associated token account...");

    let decimals = accounts.mint_account.decimals();
    let adjusted_amount = amount
        .checked_mul(10u64.pow(decimals as u32))
        .ok_or(ProgramError::ArithmeticOverflow)?;

    accounts.token_program
        .mint_to(
            &accounts.mint_account,
            &accounts.associated_token_account,
            &accounts.mint_authority,
            adjusted_amount,
        )
        .invoke()?;

    log("Token minted successfully.");
    Ok(())
}
