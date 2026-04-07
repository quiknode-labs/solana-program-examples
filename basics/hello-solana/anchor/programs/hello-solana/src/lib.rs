pub mod instructions;

use anchor_lang::prelude::*;

pub use instructions::*;

declare_id!("52rNd2KDuqHaxs2vEFfEjH2zwKScA2B9AyW8F2fAcca8");

#[program]
pub mod hello_solana {
    use super::*;

    pub fn hello(_ctx: Context<Hello>) -> Result<()> {
        hello::handler(_ctx)
    }
}
