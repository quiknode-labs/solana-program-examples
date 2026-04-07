use anchor_lang::prelude::*;
use anchor_spl::token_interface::{
    default_account_state_update, DefaultAccountStateUpdate, Mint, Token2022,
};

use crate::state::AnchorAccountState;

#[derive(Accounts)]
pub struct UpdateDefaultState<'info> {
    #[account(mut)]
    pub freeze_authority: Signer<'info>,
    #[account(
        mut,
        mint::freeze_authority = freeze_authority,
    )]
    pub mint_account: InterfaceAccount<'info, Mint>,

    pub token_program: Program<'info, Token2022>,
    pub system_program: Program<'info, System>,
}

pub fn handler(
    ctx: Context<UpdateDefaultState>,
    account_state: AnchorAccountState,
) -> Result<()> {
    // Convert AnchorAccountState to spl_token_2022::state::AccountState
    let account_state = account_state.to_spl_account_state();

    default_account_state_update(
        CpiContext::new(
            ctx.accounts.token_program.key(),
            DefaultAccountStateUpdate {
                token_program_id: ctx.accounts.token_program.to_account_info(),
                mint: ctx.accounts.mint_account.to_account_info(),
                freeze_authority: ctx.accounts.freeze_authority.to_account_info(),
            },
        ),
        &account_state,
    )?;
    Ok(())
}
