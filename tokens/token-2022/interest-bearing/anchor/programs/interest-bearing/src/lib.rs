pub mod helpers;
pub mod instructions;

use anchor_lang::prelude::*;

pub use instructions::*;

declare_id!("2tinFpSK55kJu1wjcwW1PZBDF3HrP7PaYJjDCAFJMDD8");

#[program]
pub mod interest_bearing {

    use super::*;

    pub fn initialize(ctx: Context<Initialize>, rate: i16) -> Result<()> {
        initialize::handler(ctx, rate)
    }

    pub fn update_rate(ctx: Context<UpdateRate>, rate: i16) -> Result<()> {
        update_rate::handler(ctx, rate)
    }
}
