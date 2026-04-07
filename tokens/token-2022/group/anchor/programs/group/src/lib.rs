pub mod instructions;

use anchor_lang::prelude::*;

pub use instructions::*;

declare_id!("8cToQMRhi1w6SLMLCcPniNix9eivQV3oFWwPucR5XxdR");

#[program]
pub mod group {

    use super::*;

    pub fn test_initialize_group(ctx: Context<InitializeGroup>) -> Result<()> {
        test_initialize_group::handler(ctx)
    }
}
