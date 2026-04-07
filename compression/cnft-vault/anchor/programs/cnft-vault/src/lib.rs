pub mod constants;
pub mod helpers;
pub mod instructions;

use anchor_lang::prelude::*;

pub use instructions::*;

declare_id!("Fd4iwpPWaCU8BNwGQGtvvrcvG4Tfizq3RgLm8YLBJX6D");

#[program]
pub mod cnft_vault {
    use super::*;

    pub fn withdraw_cnft<'info>(
        ctx: Context<'info, Withdraw<'info>>,
        root: [u8; 32],
        data_hash: [u8; 32],
        creator_hash: [u8; 32],
        nonce: u64,
        index: u32,
    ) -> Result<()> {
        withdraw_cnft::handler(ctx, root, data_hash, creator_hash, nonce, index)
    }

    #[allow(clippy::too_many_arguments)]
    pub fn withdraw_two_cnfts<'info>(
        ctx: Context<'info, WithdrawTwo<'info>>,
        root1: [u8; 32],
        data_hash1: [u8; 32],
        creator_hash1: [u8; 32],
        nonce1: u64,
        index1: u32,
        proof_1_length: u8,
        root2: [u8; 32],
        data_hash2: [u8; 32],
        creator_hash2: [u8; 32],
        nonce2: u64,
        index2: u32,
        _proof_2_length: u8,
    ) -> Result<()> {
        withdraw_two_cnfts::handler(
            ctx,
            root1,
            data_hash1,
            creator_hash1,
            nonce1,
            index1,
            proof_1_length,
            root2,
            data_hash2,
            creator_hash2,
            nonce2,
            index2,
            _proof_2_length,
        )
    }
}
