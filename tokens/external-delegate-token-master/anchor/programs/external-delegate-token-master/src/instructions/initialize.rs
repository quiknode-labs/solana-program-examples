use anchor_lang::prelude::*;

use crate::state::UserAccount;

#[derive(Accounts)]
pub struct Initialize<'info> {
    #[account(init, payer = authority, space = 8 + 32 + 20)]
    // Ensure this is only for user_account
    pub user_account: Account<'info, UserAccount>,
    #[account(mut)]
    pub authority: Signer<'info>, // This should remain as a signer
    pub system_program: Program<'info, System>, // Required for initialization
}

pub fn handler(ctx: Context<Initialize>) -> Result<()> {
    let user_account = &mut ctx.accounts.user_account;
    user_account.authority = ctx.accounts.authority.key();
    user_account.ethereum_address = [0; 20];
    Ok(())
}
