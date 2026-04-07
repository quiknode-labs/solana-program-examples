pub mod instructions;

use anchor_lang::prelude::*;

pub use instructions::*;

declare_id!("ALfCAvL1Vbm15QabQJ72NUi7LK7fB5YP2tDKEvvh7wS8");

#[program]
pub mod create_system_account {
    use super::*;

    pub fn create_system_account(ctx: Context<CreateSystemAccount>) -> Result<()> {
        create_system_account::handler(ctx)
    }
}
