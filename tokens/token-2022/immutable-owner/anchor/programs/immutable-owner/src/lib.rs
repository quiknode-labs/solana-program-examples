pub mod instructions;

use anchor_lang::prelude::*;

pub use instructions::*;

declare_id!("FwsJpDYJ1UkYHq6rGYD7sXY1fStiBTfLpApazzj9khS7");

#[program]
pub mod immutable_owner {
    use super::*;

    pub fn initialize(ctx: Context<Initialize>) -> Result<()> {
        initialize::handler(ctx)
    }
}
