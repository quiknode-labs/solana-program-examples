use anchor_lang::prelude::*;
use anchor_lang::solana_program::instruction::{AccountMeta, Instruction};
use borsh::BorshSerialize;

use crate::constants::{MPL_BUBBLEGUM_ID, TRANSFER_DISCRIMINATOR};

/// Instruction arguments for mpl-bubblegum Transfer, serialized with borsh
#[derive(BorshSerialize)]
pub struct TransferArgs {
    pub root: [u8; 32],
    pub data_hash: [u8; 32],
    pub creator_hash: [u8; 32],
    pub nonce: u64,
    pub index: u32,
}

/// Build a mpl-bubblegum Transfer instruction from pubkeys and args.
/// This avoids using mpl-bubblegum's CPI wrapper which requires solana-program 2.x AccountInfo.
pub fn build_transfer_instruction(
    tree_config: Pubkey,
    leaf_owner: Pubkey,
    leaf_delegate: Pubkey,
    new_leaf_owner: Pubkey,
    merkle_tree: Pubkey,
    log_wrapper: Pubkey,
    compression_program: Pubkey,
    system_program: Pubkey,
    remaining_accounts: &[AccountMeta],
    args: TransferArgs,
) -> Result<Instruction> {
    let mut accounts = Vec::with_capacity(8 + remaining_accounts.len());
    accounts.push(AccountMeta::new_readonly(tree_config, false));
    // leaf_owner is a signer (PDA signs via invoke_signed)
    accounts.push(AccountMeta::new_readonly(leaf_owner, true));
    // leaf_delegate = leaf_owner, not an additional signer
    accounts.push(AccountMeta::new_readonly(leaf_delegate, false));
    accounts.push(AccountMeta::new_readonly(new_leaf_owner, false));
    accounts.push(AccountMeta::new(merkle_tree, false));
    accounts.push(AccountMeta::new_readonly(log_wrapper, false));
    accounts.push(AccountMeta::new_readonly(compression_program, false));
    accounts.push(AccountMeta::new_readonly(system_program, false));
    accounts.extend_from_slice(remaining_accounts);

    let mut data = TRANSFER_DISCRIMINATOR.to_vec();
    args.serialize(&mut data)?;

    Ok(Instruction {
        program_id: MPL_BUBBLEGUM_ID,
        accounts,
        data,
    })
}
