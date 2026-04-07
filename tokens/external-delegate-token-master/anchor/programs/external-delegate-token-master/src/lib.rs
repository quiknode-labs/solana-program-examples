pub mod error;
pub mod helpers;
pub mod instructions;
pub mod state;

use anchor_lang::prelude::*;

pub use instructions::*;
pub use state::*;

declare_id!("85ucSW6NvhoAUYMr9QBXggsGLh3h2EqdWSDuGLzPetFd");

#[program]
pub mod external_delegate_token_master {
    use super::*;

    pub fn initialize(ctx: Context<Initialize>) -> Result<()> {
        initialize::handler(ctx)
    }

    pub fn set_ethereum_address(
        ctx: Context<SetEthereumAddress>,
        ethereum_address: [u8; 20],
    ) -> Result<()> {
        set_ethereum_address::handler(ctx, ethereum_address)
    }

    pub fn transfer_tokens(
        ctx: Context<TransferTokens>,
        amount: u64,
        signature: [u8; 65],
        message: [u8; 32],
    ) -> Result<()> {
        transfer_tokens::handler(ctx, amount, signature, message)
    }

    pub fn authority_transfer(ctx: Context<AuthorityTransfer>, amount: u64) -> Result<()> {
        authority_transfer::handler(ctx, amount)
    }
}
