use anchor_lang::prelude::*;

use crate::state::WhiteList;

#[derive(Accounts)]
pub struct AddToWhiteList<'info> {
    /// CHECK: New account to add to white list
    #[account()]
    pub new_account: UncheckedAccount<'info>,
    #[account(
        mut,
        seeds = [b"white_list"],
        bump
    )]
    pub white_list: Account<'info, WhiteList>,
    #[account(mut)]
    pub signer: Signer<'info>,
}

pub fn handler(ctx: Context<AddToWhiteList>) -> Result<()> {
    if ctx.accounts.white_list.authority != ctx.accounts.signer.key() {
        panic!("Only the authority can add to the white list!");
    }

    ctx.accounts
        .white_list
        .white_list
        .push(ctx.accounts.new_account.key());
    msg!(
        "New account white listed! {0}",
        ctx.accounts.new_account.key().to_string()
    );
    msg!(
        "White list length! {0}",
        ctx.accounts.white_list.white_list.len()
    );

    Ok(())
}
