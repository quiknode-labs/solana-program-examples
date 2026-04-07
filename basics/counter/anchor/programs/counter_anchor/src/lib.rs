pub mod instructions;
pub mod state;

use anchor_lang::prelude::*;

pub use instructions::*;
pub use state::*;

declare_id!("9nG25nFpLsycMERePF627oKHm5cHvNyv2TbfiFck32EP");

#[program]
pub mod counter_anchor {
    use super::*;

    pub fn initialize_counter(_ctx: Context<InitializeCounter>) -> Result<()> {
        initialize_counter::handler(_ctx)
    }

    pub fn increment(ctx: Context<Increment>) -> Result<()> {
        increment::handler(ctx)
    }
}
