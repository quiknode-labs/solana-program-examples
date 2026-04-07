pub mod instructions;

use anchor_lang::prelude::*;

pub use instructions::*;

declare_id!("5bz3T8zeTXMGEwVhuFBKmyZk1Wqhfs1K39kfDXnLLmwG");

#[program]
pub mod transfer_sol {
    use super::*;

    pub fn transfer_sol_with_cpi(ctx: Context<TransferSolWithCpi>, amount: u64) -> Result<()> {
        transfer_sol_with_cpi::handler(ctx, amount)
    }

    pub fn transfer_sol_with_program(
        ctx: Context<TransferSolWithProgram>,
        amount: u64,
    ) -> Result<()> {
        transfer_sol_with_program::handler(ctx, amount)
    }
}
