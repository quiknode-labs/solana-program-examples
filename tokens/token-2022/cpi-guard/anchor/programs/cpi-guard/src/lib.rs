pub mod instructions;

use anchor_lang::prelude::*;

pub use instructions::*;

declare_id!("PWPfSkM5ntHyS68woBexCrrDVE23AdtdQbUoyx5q7GR");

#[program]
pub mod cpi_guard {
    use super::*;

    pub fn cpi_transfer(ctx: Context<CpiTransfer>) -> Result<()> {
        cpi_transfer::handler(ctx)
    }
}
