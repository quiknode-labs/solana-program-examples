pub mod instructions;
pub mod state;

use anchor_lang::prelude::*;

pub use instructions::*;
pub use state::*;

declare_id!("3aP7aVaJck5pUzbh3t6cqyCPgk6otbiyHsNsXHzeNDE5");

#[program]
pub mod lever {
    use super::*;

    pub fn initialize(_ctx: Context<InitializeLever>) -> Result<()> {
        initialize::handler(_ctx)
    }

    pub fn switch_power(ctx: Context<SetPowerStatus>, name: String) -> Result<()> {
        switch_power::handler(ctx, name)
    }
}
