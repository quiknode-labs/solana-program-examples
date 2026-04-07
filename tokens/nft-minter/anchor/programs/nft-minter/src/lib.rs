pub mod instructions;

use anchor_lang::prelude::*;

pub use instructions::*;

declare_id!("H7AFrTXDtmpPoc2kHD8hkEF1i8Pbhi17QUczbkWYYFBV");

#[program]
pub mod nft_minter {
    use super::*;

    pub fn mint_nft(
        ctx: Context<CreateToken>,
        nft_name: String,
        nft_symbol: String,
        nft_uri: String,
    ) -> Result<()> {
        mint_nft::handler(ctx, nft_name, nft_symbol, nft_uri)
    }
}
