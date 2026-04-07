use anchor_lang::prelude::*;
use anchor_lang::solana_program::{
    instruction::AccountMeta,
    program::invoke_signed,
};

use crate::constants::SPLCompression;
use crate::helpers::{build_transfer_instruction, TransferArgs};

#[derive(Accounts)]
pub struct Withdraw<'info> {
    #[account(mut)]
    #[account(
        seeds = [merkle_tree.key().as_ref()],
        bump,
        seeds::program = bubblegum_program.key()
    )]
    /// CHECK: This account is modified in the downstream program
    pub tree_authority: UncheckedAccount<'info>,
    #[account(
        seeds = [b"cNFT-vault"],
        bump,
    )]
    /// CHECK: This account doesnt even exist (it is just the pda to sign)
    pub leaf_owner: UncheckedAccount<'info>,
    /// CHECK: This account is neither written to nor read from.
    pub new_leaf_owner: UncheckedAccount<'info>,
    #[account(mut)]
    /// CHECK: This account is modified in the downstream program
    pub merkle_tree: UncheckedAccount<'info>,
    /// CHECK: This account is neither written to nor read from.
    pub log_wrapper: UncheckedAccount<'info>,
    pub compression_program: Program<'info, SPLCompression>,
    /// CHECK: This account is neither written to nor read from.
    pub bubblegum_program: UncheckedAccount<'info>,
    pub system_program: Program<'info, System>,
}

pub fn handler<'info>(
    ctx: Context<'info, Withdraw<'info>>,
    root: [u8; 32],
    data_hash: [u8; 32],
    creator_hash: [u8; 32],
    nonce: u64,
    index: u32,
) -> Result<()> {
    msg!(
        "attempting to send nft {} from tree {}",
        index,
        ctx.accounts.merkle_tree.key()
    );

    let proof_metas: Vec<AccountMeta> = ctx
        .remaining_accounts
        .iter()
        .map(|acc| AccountMeta::new_readonly(acc.key(), false))
        .collect();

    let instruction = build_transfer_instruction(
        ctx.accounts.tree_authority.key(),
        ctx.accounts.leaf_owner.key(),
        ctx.accounts.leaf_owner.key(),
        ctx.accounts.new_leaf_owner.key(),
        ctx.accounts.merkle_tree.key(),
        ctx.accounts.log_wrapper.key(),
        ctx.accounts.compression_program.key(),
        ctx.accounts.system_program.key(),
        &proof_metas,
        TransferArgs {
            root,
            data_hash,
            creator_hash,
            nonce,
            index,
        },
    )?;

    // Gather all account infos for the CPI
    let mut account_infos = vec![
        ctx.accounts.bubblegum_program.to_account_info(),
        ctx.accounts.tree_authority.to_account_info(),
        ctx.accounts.leaf_owner.to_account_info(),
        ctx.accounts.new_leaf_owner.to_account_info(),
        ctx.accounts.merkle_tree.to_account_info(),
        ctx.accounts.log_wrapper.to_account_info(),
        ctx.accounts.compression_program.to_account_info(),
        ctx.accounts.system_program.to_account_info(),
    ];
    for acc in ctx.remaining_accounts.iter() {
        account_infos.push(acc.to_account_info());
    }

    invoke_signed(
        &instruction,
        &account_infos,
        &[&[b"cNFT-vault", &[ctx.bumps.leaf_owner]]],
    )?;

    Ok(())
}
