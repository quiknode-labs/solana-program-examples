pub mod instructions;
pub mod state;

use anchor_lang::prelude::*;

pub use instructions::*;
pub use state::*;

declare_id!("3efkmFTva1SZmFPt7E2nTe6SkCLUfuzayXgfjsMJr5Ac");

#[program]
pub mod rent_example {
    use super::*;

    pub fn create_system_account(
        ctx: Context<CreateSystemAccount>,
        address_data: AddressData,
    ) -> Result<()> {
        create_system_account::handler(ctx, address_data)
    }
}
