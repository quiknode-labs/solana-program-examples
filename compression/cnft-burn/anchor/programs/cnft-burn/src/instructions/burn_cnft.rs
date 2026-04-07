use anchor_lang::prelude::*;
use anchor_lang::solana_program::{
    instruction::{AccountMeta, Instruction},
    program::invoke,
};
use borsh::BorshSerialize;

use crate::constants::{BURN_DISCRIMINATOR, MPL_BUBBLEGUM_ID, SPLCompression};

/// Instruction arguments for mpl-bubblegum Burn, serialized with borsh
#[derive(BorshSerialize)]
struct BurnArgs {
    root: [u8; 32],
    data_hash: [u8; 32],
    creator_hash: [u8; 32],
    nonce: u64,
    index: u32,
}

#[derive(Accounts)]
pub struct BurnCnft<'info> {
    #[account(mut)]
    pub leaf_owner: Signer<'info>,
    #[account(mut)]
    #[account(
        seeds = [merkle_tree.key().as_ref()],
        bump,
        seeds::program = bubblegum_program.key()
    )]
    /// CHECK: This account is modified in the downstream program
    pub tree_authority: UncheckedAccount<'info>,
    #[account(mut)]
    /// CHECK: This account is neither written to nor read from.
    pub merkle_tree: UncheckedAccount<'info>,
    /// CHECK: This account is neither written to nor read from.
    pub log_wrapper: UncheckedAccount<'info>,
    pub compression_program: Program<'info, SPLCompression>,
    /// CHECK: This account is neither written to nor read from.
    pub bubblegum_program: UncheckedAccount<'info>,
    pub system_program: Program<'info, System>,
}

pub fn handler<'info>(
    ctx: Context<'info, BurnCnft<'info>>,
    root: [u8; 32],
    data_hash: [u8; 32],
    creator_hash: [u8; 32],
    nonce: u64,
    index: u32,
) -> Result<()> {
    // Build instruction data: discriminator + borsh-serialized args
    let args = BurnArgs {
        root,
        data_hash,
        creator_hash,
        nonce,
        index,
    };
    let mut data = BURN_DISCRIMINATOR.to_vec();
    args.serialize(&mut data)?;

    // Build account metas matching mpl-bubblegum Burn instruction layout
    let mut accounts = Vec::with_capacity(7 + ctx.remaining_accounts.len());
    accounts.push(AccountMeta::new_readonly(
        ctx.accounts.tree_authority.key(),
        false,
    ));
    accounts.push(AccountMeta::new_readonly(
        ctx.accounts.leaf_owner.key(),
        true,
    ));
    // leaf_delegate = leaf_owner, not a signer in this call
    accounts.push(AccountMeta::new_readonly(
        ctx.accounts.leaf_owner.key(),
        false,
    ));
    accounts.push(AccountMeta::new(ctx.accounts.merkle_tree.key(), false));
    accounts.push(AccountMeta::new_readonly(
        ctx.accounts.log_wrapper.key(),
        false,
    ));
    accounts.push(AccountMeta::new_readonly(
        ctx.accounts.compression_program.key(),
        false,
    ));
    accounts.push(AccountMeta::new_readonly(
        ctx.accounts.system_program.key(),
        false,
    ));
    // Append remaining accounts (proof nodes)
    for acc in ctx.remaining_accounts.iter() {
        accounts.push(AccountMeta::new_readonly(acc.key(), false));
    }

    let instruction = Instruction {
        program_id: MPL_BUBBLEGUM_ID,
        accounts,
        data,
    };

    // Gather all account infos for the CPI
    let mut account_infos = vec![
        ctx.accounts.bubblegum_program.to_account_info(),
        ctx.accounts.tree_authority.to_account_info(),
        ctx.accounts.leaf_owner.to_account_info(),
        ctx.accounts.merkle_tree.to_account_info(),
        ctx.accounts.log_wrapper.to_account_info(),
        ctx.accounts.compression_program.to_account_info(),
        ctx.accounts.system_program.to_account_info(),
    ];
    for acc in ctx.remaining_accounts.iter() {
        account_infos.push(acc.to_account_info());
    }

    invoke(&instruction, &account_infos)?;

    Ok(())
}
