pub mod instructions;

use anchor_lang::prelude::*;

pub use instructions::*;

declare_id!("g63s55bPYTvuuXFRMCzDaKTjCnXhDPNmwjafnQUNexa");

#[program]
pub mod permanent_delegate {
    use super::*;

    pub fn initialize(ctx: Context<Initialize>) -> Result<()> {
        initialize::handler(ctx)
    }
}
