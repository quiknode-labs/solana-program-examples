pub mod instructions;

use anchor_lang::prelude::*;

pub use instructions::*;

declare_id!("C3zvHFYvrYgydPXv6vPZkcZvx24YE6pTUTZmtQJnAZQs");

#[program]
pub mod memo_transfer {
    use super::*;

    pub fn initialize(ctx: Context<Initialize>) -> Result<()> {
        initialize::handler(ctx)
    }

    pub fn disable(ctx: Context<Disable>) -> Result<()> {
        disable::handler(ctx)
    }
}
