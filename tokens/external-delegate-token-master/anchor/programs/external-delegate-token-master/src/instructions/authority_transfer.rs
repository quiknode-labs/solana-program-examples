use anchor_lang::prelude::*;
use anchor_spl::token;
use anchor_spl::token::{Token, TokenAccount, Transfer};

use crate::state::UserAccount;

#[derive(Accounts)]
pub struct AuthorityTransfer<'info> {
    #[account(has_one = authority)]
    pub user_account: Account<'info, UserAccount>,
    pub authority: Signer<'info>,
    #[account(mut)]
    pub user_token_account: Account<'info, TokenAccount>,
    #[account(mut)]
    pub recipient_token_account: Account<'info, TokenAccount>,
    #[account(
        seeds = [user_account.key().as_ref()],
        bump,
    )]
    pub user_pda: SystemAccount<'info>,
    pub token_program: Program<'info, Token>,
}

pub fn handler(ctx: Context<AuthorityTransfer>, amount: u64) -> Result<()> {
    // Transfer tokens
    let transfer_instruction = Transfer {
        from: ctx.accounts.user_token_account.to_account_info(),
        to: ctx.accounts.recipient_token_account.to_account_info(),
        authority: ctx.accounts.user_pda.to_account_info(),
    };

    token::transfer(
        CpiContext::new_with_signer(
            ctx.accounts.token_program.key(),
            transfer_instruction,
            &[&[
                ctx.accounts.user_account.key().as_ref(),
                &[ctx.bumps.user_pda],
            ]],
        ),
        amount,
    )?;

    Ok(())
}
