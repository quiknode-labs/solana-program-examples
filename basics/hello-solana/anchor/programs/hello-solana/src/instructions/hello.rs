use anchor_lang::prelude::*;

use crate::id;

#[derive(Accounts)]
pub struct Hello {}

pub fn handler(_ctx: Context<Hello>) -> Result<()> {
    msg!("Hello, Solana!");

    msg!("Our program's Program ID: {}", &id());

    Ok(())
}
