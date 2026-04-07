pub mod instructions;

use anchor_lang::prelude::*;

pub use instructions::*;

declare_id!("Ezf9iNnBPVNr596tK59pTP1MdDWAS2VvNjyiK5gkbSEX");

#[program]
pub mod mint_nft {
    use super::*;

    pub fn create_collection(ctx: Context<CreateCollection>) -> Result<()> {
        instructions::create_collection::handler(ctx)
    }

    pub fn mint_nft(ctx: Context<MintNFT>) -> Result<()> {
        instructions::mint_nft::handler(ctx)
    }

    pub fn verify_collection(ctx: Context<VerifyCollectionMint>) -> Result<()> {
        instructions::verify_collection::handler(ctx)
    }
}
