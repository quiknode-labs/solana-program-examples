use std::cell::RefMut;

use anchor_lang::prelude::*;
use anchor_spl::{
    associated_token::AssociatedToken,
    token::Token,
    token_2022::spl_token_2022::{
        extension::{
            transfer_hook::TransferHookAccount, BaseStateWithExtensionsMut,
            PodStateWithExtensionsMut,
        },
        pod::PodAccount,
    },
    token_interface::{transfer_checked, Mint, TokenAccount, TransferChecked},
};

use crate::error::TransferError;
use crate::state::CounterAccount;

// Order of accounts matters for this struct.
// The first 4 accounts are the accounts required for token transfer (source, mint, destination, owner)
// Remaining accounts are the extra accounts required from the ExtraAccountMetaList account
// These accounts are provided via CPI to this program from the token2022 program
//
// Box<InterfaceAccount> used for source_token, destination_token, wsol_mint,
// delegate_wsol_token_account, and sender_wsol_token_account to avoid exceeding
// the 4096-byte BPF stack frame limit in try_accounts deserialization.
// This struct has 12 accounts — without Box, the generated code uses ~4160 bytes of stack.
#[derive(Accounts)]
pub struct TransferHook<'info> {
    #[account(token::mint = mint, token::authority = owner)]
    pub source_token: Box<InterfaceAccount<'info, TokenAccount>>,
    pub mint: Box<InterfaceAccount<'info, Mint>>,
    #[account(token::mint = mint)]
    pub destination_token: Box<InterfaceAccount<'info, TokenAccount>>,
    /// CHECK: source token account owner, can be SystemAccount or PDA owned by another program
    pub owner: UncheckedAccount<'info>,
    /// CHECK: ExtraAccountMetaList Account,
    #[account(seeds = [b"extra-account-metas", mint.key().as_ref()], bump)]
    pub extra_account_meta_list: UncheckedAccount<'info>,
    pub wsol_mint: Box<InterfaceAccount<'info, Mint>>,
    pub token_program: Program<'info, Token>,
    pub associated_token_program: Program<'info, AssociatedToken>,
    #[account(
        mut,
        seeds = [b"delegate"],
        bump
    )]
    pub delegate: SystemAccount<'info>,
    #[account(
        mut,
        token::mint = wsol_mint,
        token::authority = delegate,
    )]
    pub delegate_wsol_token_account: Box<InterfaceAccount<'info, TokenAccount>>,
    #[account(
        mut,
        token::mint = wsol_mint,
        token::authority = owner,
    )]
    pub sender_wsol_token_account: Box<InterfaceAccount<'info, TokenAccount>>,
    #[account(seeds = [b"counter"], bump)]
    pub counter_account: Account<'info, CounterAccount>,
}

fn check_is_transferring(ctx: &Context<TransferHook>) -> Result<()> {
    let source_token_info = ctx.accounts.source_token.to_account_info();
    let mut account_data_ref: RefMut<&mut [u8]> = source_token_info.try_borrow_mut_data()?;
    let mut account = PodStateWithExtensionsMut::<PodAccount>::unpack(*account_data_ref)
        .map_err(|_| ProgramError::InvalidAccountData)?;
    let account_extension = account
        .get_extension_mut::<TransferHookAccount>()
        .map_err(|_| ProgramError::InvalidAccountData)?;

    if !bool::from(account_extension.transferring) {
        return err!(TransferError::IsNotCurrentlyTransferring);
    }

    Ok(())
}

pub fn handler(ctx: Context<TransferHook>, amount: u64) -> Result<()> {
    // Fail this instruction if it is not called from within a transfer hook
    check_is_transferring(&ctx)?;

    if amount > 50 {
        msg!("The amount is too big {0}", amount);
    }

    ctx.accounts.counter_account.counter += 1;

    msg!(
        "This token has been transferred {0} times",
        ctx.accounts.counter_account.counter
    );

    msg!(
        "Is writable mint {0}",
        ctx.accounts.mint.to_account_info().is_writable
    );
    msg!(
        "Is destination mint {0}",
        ctx.accounts.destination_token.to_account_info().is_writable
    );
    msg!(
        "Is source mint {0}",
        ctx.accounts.source_token.to_account_info().is_writable
    );

    let signer_seeds: &[&[&[u8]]] = &[&[b"delegate", &[ctx.bumps.delegate]]];

    // Transfer WSOL from sender to delegate token account using delegate PDA
    // transfer lamports amount equal to token transfer amount
    transfer_checked(
        CpiContext::new(
            ctx.accounts.token_program.key(),
            TransferChecked {
                from: ctx.accounts.sender_wsol_token_account.to_account_info(),
                mint: ctx.accounts.wsol_mint.to_account_info(),
                to: ctx.accounts.delegate_wsol_token_account.to_account_info(),
                authority: ctx.accounts.delegate.to_account_info(),
            },
        )
        .with_signer(signer_seeds),
        amount,
        ctx.accounts.wsol_mint.decimals,
    )?;
    Ok(())
}
