use anchor_lang::prelude::*;

mod instructions;
use instructions::*;

declare_id!("AcfQLsYKuzprcCNH1n96pKKgAbAnZchwpbr3gbVN742n");

#[program]
pub mod mint_close_authority {
    use super::*;

    pub fn initialize(context: Context<Initialize>) -> Result<()> {
        instructions::initialize::handler(context)
    }

    pub fn close(context: Context<Close>) -> Result<()> {
        instructions::close::handler(context)
    }
}
