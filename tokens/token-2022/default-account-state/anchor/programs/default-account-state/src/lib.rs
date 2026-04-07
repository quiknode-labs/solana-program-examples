pub mod instructions;
pub mod state;

use anchor_lang::prelude::*;

pub use instructions::*;
pub use state::*;

declare_id!("HVMebUevbwW8QbUxSyCtNwsJcpgWj9jVkW38g69hTHRT");

#[program]
pub mod default_account_state {
    use super::*;

    pub fn initialize(ctx: Context<Initialize>) -> Result<()> {
        initialize::handler(ctx)
    }

    pub fn update_default_state(
        ctx: Context<UpdateDefaultState>,
        account_state: AnchorAccountState,
    ) -> Result<()> {
        update_default_state::handler(ctx, account_state)
    }
}
