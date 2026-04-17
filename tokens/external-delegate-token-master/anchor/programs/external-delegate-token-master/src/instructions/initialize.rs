use anchor_lang::prelude::*;

use crate::UserAccount;

#[derive(Accounts)]
pub struct Initialize<'info> {
    #[account(
        init,
        payer = authority,
        space = UserAccount::DISCRIMINATOR.len() + UserAccount::INIT_SPACE,
    )]
    // Ensure this is only for user_account
    pub user_account: Account<'info, UserAccount>,
    #[account(mut)]
    pub authority: Signer<'info>, // This should remain as a signer
    pub system_program: Program<'info, System>, // Required for initialization
}

pub fn handler(mut context: Context<Initialize>) -> Result<()> {
    let user_account = &mut context.accounts.user_account;
    user_account.authority = context.accounts.authority.key();
    user_account.ethereum_address = [0; 20];
    Ok(())
}
