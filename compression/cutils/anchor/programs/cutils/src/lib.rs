pub mod bubblegum_types;
pub mod constants;
pub mod instructions;
pub mod state;

use anchor_lang::prelude::*;

pub use instructions::*;
pub use state::*;

declare_id!("BuFyrgRYzg2nPhqYrxZ7d9uYUs4VXtxH71U8EcoAfTQZ");

#[program]
pub mod cutils {
    use super::*;

    #[access_control(ctx.accounts.validate(&ctx, &params))]
    pub fn mint<'info>(
        ctx: Context<'info, Mint<'info>>,
        params: MintParams,
    ) -> Result<()> {
        mint::handler(ctx, params)
    }

    #[access_control(ctx.accounts.validate(&ctx, &params))]
    pub fn verify<'info>(
        ctx: Context<'info, Verify<'info>>,
        params: VerifyParams,
    ) -> Result<()> {
        verify::handler(ctx, &params)
    }
}
