use anchor_lang::prelude::*;

use crate::state::Counter;

#[derive(Accounts)]
pub struct InitializeCounter<'info> {
    #[account(mut)]
    pub payer: Signer<'info>,

    #[account(
        init,
        space = 8 + Counter::INIT_SPACE,
        payer = payer
    )]
    pub counter: Account<'info, Counter>,
    pub system_program: Program<'info, System>,
}

pub fn handler(_ctx: Context<InitializeCounter>) -> Result<()> {
    Ok(())
}
