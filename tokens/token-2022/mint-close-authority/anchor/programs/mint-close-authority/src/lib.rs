pub mod instructions;

use anchor_lang::prelude::*;

pub use instructions::*;

declare_id!("2wT6WQQqD4P35mDQJAWbWAMzFfCeLsYEELTde6jLQyKN");

#[program]
pub mod mint_close_authority {
    use super::*;

    pub fn initialize(ctx: Context<Initialize>) -> Result<()> {
        initialize::handler(ctx)
    }

    pub fn close(ctx: Context<Close>) -> Result<()> {
        close::handler(ctx)
    }
}
