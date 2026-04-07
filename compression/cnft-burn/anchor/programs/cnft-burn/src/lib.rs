pub mod constants;
pub mod instructions;

use anchor_lang::prelude::*;

pub use instructions::*;

declare_id!("HELUXAsLsF3wVoehtbcvQyUafUdMiFxbJG6MPiCRnzwm");

#[program]
pub mod cnft_burn {
    use super::*;

    pub fn burn_cnft<'info>(
        ctx: Context<'info, BurnCnft<'info>>,
        root: [u8; 32],
        data_hash: [u8; 32],
        creator_hash: [u8; 32],
        nonce: u64,
        index: u32,
    ) -> Result<()> {
        burn_cnft::handler(ctx, root, data_hash, creator_hash, nonce, index)
    }
}
