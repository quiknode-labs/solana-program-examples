pub mod instructions;
pub mod state;

use anchor_lang::prelude::*;

pub use instructions::*;
pub use state::*;

declare_id!("4JzDy7ZPoTzSxWCFu8dmxFqEyJzLeXacYJ5xDxopt5vz");

#[program]
pub mod anchor_realloc {
    use super::*;

    pub fn initialize(ctx: Context<Initialize>, input: String) -> Result<()> {
        initialize::handler(ctx, input)
    }

    pub fn update(ctx: Context<Update>, input: String) -> Result<()> {
        update::handler(ctx, input)
    }
}
