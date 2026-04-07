use anchor_lang::prelude::*;

use crate::state::UserAccount;

#[derive(Accounts)]
pub struct SetEthereumAddress<'info> {
    #[account(mut, has_one = authority)]
    pub user_account: Account<'info, UserAccount>,
    pub authority: Signer<'info>,
}

pub fn handler(
    ctx: Context<SetEthereumAddress>,
    ethereum_address: [u8; 20],
) -> Result<()> {
    let user_account = &mut ctx.accounts.user_account;
    user_account.ethereum_address = ethereum_address;
    Ok(())
}
