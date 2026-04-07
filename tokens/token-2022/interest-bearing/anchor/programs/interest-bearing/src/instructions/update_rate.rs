use anchor_lang::prelude::*;
use anchor_spl::token_interface::{
    interest_bearing_mint_update_rate, InterestBearingMintUpdateRate, Mint, Token2022,
};

use crate::helpers::check_mint_data;

#[derive(Accounts)]
pub struct UpdateRate<'info> {
    #[account(mut)]
    pub authority: Signer<'info>,
    #[account(mut)]
    pub mint_account: InterfaceAccount<'info, Mint>,

    pub token_program: Program<'info, Token2022>,
    pub system_program: Program<'info, System>,
}

pub fn handler(ctx: Context<UpdateRate>, rate: i16) -> Result<()> {
    interest_bearing_mint_update_rate(
        CpiContext::new(
            ctx.accounts.token_program.key(),
            InterestBearingMintUpdateRate {
                token_program_id: ctx.accounts.token_program.to_account_info(),
                mint: ctx.accounts.mint_account.to_account_info(),
                rate_authority: ctx.accounts.authority.to_account_info(),
            },
        ),
        rate,
    )?;

    check_mint_data(
        &ctx.accounts.mint_account.to_account_info(),
        &ctx.accounts.authority.key(),
    )?;
    Ok(())
}
