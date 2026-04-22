#![no_std]

use quasar_lang::prelude::*;
use quasar_spl::{Mint, Token, TokenCpi};

#[cfg(test)]
mod tests;

declare_id!("22222222222222222222222222222222222222222222");

/// Demonstrates creating a mint, minting tokens, and transferring between accounts.
///
/// The Anchor version uses Metaplex for onchain metadata. Quasar does not have
/// a Metaplex integration crate, so this example focuses on the core SPL Token
/// operations: minting and transferring.
#[program]
mod quasar_transfer_tokens {
    use super::*;

    /// Mint tokens to a recipient's token account.
    #[instruction(discriminator = 0)]
    pub fn mint_tokens(ctx: Ctx<MintTokens>, amount: u64) -> Result<(), ProgramError> {
        ctx.accounts.mint_tokens(amount)
    }

    /// Transfer tokens from sender to recipient.
    #[instruction(discriminator = 1)]
    pub fn transfer_tokens(ctx: Ctx<TransferTokens>, amount: u64) -> Result<(), ProgramError> {
        ctx.accounts.transfer_tokens(amount)
    }
}

/// Accounts for minting tokens to a recipient.
#[derive(Accounts)]
pub struct MintTokens {
    #[account(mut)]
    pub mint_authority: Signer,
    #[account(mut)]
    pub mint: Account<Mint>,
    /// The recipient's token account. Must already exist.
    #[account(mut)]
    pub recipient_token_account: Account<Token>,
    pub token_program: Program<Token>,
}

impl MintTokens {
    #[inline(always)]
    pub fn mint_tokens(&mut self, amount: u64) -> Result<(), ProgramError> {
        self.token_program
            .mint_to(&self.mint, &self.recipient_token_account, &self.mint_authority, amount)
            .invoke()
    }
}

/// Accounts for transferring tokens between two token accounts.
#[derive(Accounts)]
pub struct TransferTokens {
    #[account(mut)]
    pub sender: Signer,
    #[account(mut)]
    pub sender_token_account: Account<Token>,
    #[account(mut)]
    pub recipient_token_account: Account<Token>,
    pub token_program: Program<Token>,
}

impl TransferTokens {
    #[inline(always)]
    pub fn transfer_tokens(&mut self, amount: u64) -> Result<(), ProgramError> {
        self.token_program
            .transfer(&self.sender_token_account, &self.recipient_token_account, &self.sender, amount)
            .invoke()
    }
}
