pub mod instructions;

use anchor_lang::prelude::*;

pub use instructions::*;

declare_id!("9GpYDDJUM7CCrruDsg9v1aff5BouEzGfXifu8Woyhnbn");

#[program]
pub mod non_transferable {
    use super::*;

    pub fn initialize(ctx: Context<Initialize>) -> Result<()> {
        initialize::handler(ctx)
    }
}
