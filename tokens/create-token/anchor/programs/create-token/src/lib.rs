pub mod instructions;

use anchor_lang::prelude::*;

pub use instructions::*;

declare_id!("Ex6CEJiwPiCotaNYmrjLSKQTr27tXrrA7981XZuQHQWe");

#[program]
pub mod create_token {
    use super::*;

    pub fn create_token_mint(
        ctx: Context<CreateTokenMint>,
        _token_decimals: u8,
        token_name: String,
        token_symbol: String,
        token_uri: String,
    ) -> Result<()> {
        create_token_mint::handler(ctx, _token_decimals, token_name, token_symbol, token_uri)
    }
}
