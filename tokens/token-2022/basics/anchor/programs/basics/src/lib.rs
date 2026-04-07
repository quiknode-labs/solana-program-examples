pub mod instructions;

use anchor_lang::prelude::*;

pub use instructions::*;

declare_id!("Gd65x65gsd55JTEbpbB7B5ukg8Q6hctXzyGcWiTmzxjK");

#[program]
pub mod anchor {

    use super::*;

    pub fn create_token(_ctx: Context<CreateToken>, _token_name: String) -> Result<()> {
        create_token::handler(_ctx, _token_name)
    }
    pub fn create_token_account(_ctx: Context<CreateTokenAccount>) -> Result<()> {
        create_token_account::handler(_ctx)
    }
    pub fn create_associated_token_account(
        _ctx: Context<CreateAssociatedTokenAccount>,
    ) -> Result<()> {
        create_associated_token_account::handler(_ctx)
    }
    pub fn transfer_token(ctx: Context<TransferToken>, amount: u64) -> Result<()> {
        transfer_token::handler(ctx, amount)
    }
    pub fn mint_token(ctx: Context<MintToken>, amount: u64) -> Result<()> {
        mint_token::handler(ctx, amount)
    }
}
