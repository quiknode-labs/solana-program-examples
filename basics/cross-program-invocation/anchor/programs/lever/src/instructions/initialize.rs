use anchor_lang::prelude::*;

use crate::state::PowerStatus;

#[derive(Accounts)]
pub struct InitializeLever<'info> {
    #[account(init, payer = user, space = 8 + 8)]
    pub power: Account<'info, PowerStatus>,
    #[account(mut)]
    pub user: Signer<'info>,
    pub system_program: Program<'info, System>,
}

pub fn handler(_ctx: Context<InitializeLever>) -> Result<()> {
    Ok(())
}
