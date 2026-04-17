use anchor_lang::prelude::*;

mod instructions;
use instructions::*;

declare_id!("5BQyC7y2Pc283woThq11uZRqsgcRbBRLKz4yQ8BJadi2");

#[program]
pub mod memo_transfer {
    use super::*;

    pub fn initialize(context: Context<Initialize>) -> Result<()> {
        instructions::initialize::handler(context)
    }

    pub fn disable(context: Context<Disable>) -> Result<()> {
        instructions::disable::handler(context)
    }
}
