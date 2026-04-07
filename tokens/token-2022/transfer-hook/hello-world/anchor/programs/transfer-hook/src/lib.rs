pub mod error;
pub mod instructions;

use anchor_lang::prelude::*;

pub use instructions::*;

use spl_discriminator::SplDiscriminate;
use spl_transfer_hook_interface::instruction::{
    ExecuteInstruction, InitializeExtraAccountMetaListInstruction,
};

declare_id!("2CnfERh8AAgu7hY5Pz6nNSjU5UihtfH8YrHAbXBoCwKg");

#[program]
pub mod transfer_hook {
    use super::*;

    // create a mint account that specifies this program as the transfer hook program
    pub fn initialize(ctx: Context<Initialize>, _decimals: u8) -> Result<()> {
        initialize::handler(ctx, _decimals)
    }

    #[instruction(discriminator = InitializeExtraAccountMetaListInstruction::SPL_DISCRIMINATOR_SLICE)]
    pub fn initialize_extra_account_meta_list(
        ctx: Context<InitializeExtraAccountMetaList>,
    ) -> Result<()> {
        initialize_extra_account_meta_list::handler(ctx)
    }

    #[instruction(discriminator = ExecuteInstruction::SPL_DISCRIMINATOR_SLICE)]
    pub fn transfer_hook(ctx: Context<TransferHook>, _amount: u64) -> Result<()> {
        instructions::transfer_hook::handler(ctx, _amount)
    }
}
