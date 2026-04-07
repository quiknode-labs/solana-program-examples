use anchor_lang::prelude::*;
use anchor_spl::token;
use anchor_spl::token::{Token, TokenAccount, Transfer};

use crate::error::ErrorCode;
use crate::helpers::verify_ethereum_signature;
use crate::state::UserAccount;

#[derive(Accounts)]
pub struct TransferTokens<'info> {
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

pub fn handler(
    ctx: Context<TransferTokens>,
    amount: u64,
    signature: [u8; 65],
    message: [u8; 32],
) -> Result<()> {
    let user_account = &ctx.accounts.user_account;

    if !verify_ethereum_signature(&user_account.ethereum_address, &message, &signature) {
        return Err(ErrorCode::InvalidSignature.into());
    }

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
            &[&[user_account.key().as_ref(), &[ctx.bumps.user_pda]]],
        ),
        amount,
    )?;

    Ok(())
}
