use {
    crate::state::{Amm, Pool},
    quasar_lang::prelude::*,
    quasar_spl::{Mint, Token},
};

/// Accounts for creating a new liquidity pool.
///
/// Seeds are based on account addresses: pool = [amm, mint_a, mint_b],
/// pool_authority = [amm, mint_a, mint_b, "authority"],
/// mint_liquidity = [amm, mint_a, mint_b, "liquidity"].
#[derive(Accounts)]
pub struct CreatePool {
    #[account(seeds = [b"amm"], bump)]
    pub amm: Account<Amm>,
    #[account(mut, init, payer = payer, seeds = [amm, mint_a, mint_b], bump)]
    pub pool: Account<Pool>,
    /// Pool authority PDA — signs for pool token operations.
    #[account(seeds = [amm, mint_a, mint_b, crate::AUTHORITY_SEED], bump)]
    pub pool_authority: UncheckedAccount,
    /// Liquidity token mint — created at a PDA.
    #[account(
        mut,
        init,
        payer = payer,
        seeds = [amm, mint_a, mint_b, crate::LIQUIDITY_SEED],
        bump,
        mint::decimals = 6,
        mint::authority = pool_authority,
    )]
    pub mint_liquidity: Account<Mint>,
    pub mint_a: Account<Mint>,
    pub mint_b: Account<Mint>,
    /// Pool's token A account.
    #[account(mut, init_if_needed, payer = payer, token::mint = mint_a, token::authority = pool_authority)]
    pub pool_account_a: Account<Token>,
    /// Pool's token B account.
    #[account(mut, init_if_needed, payer = payer, token::mint = mint_b, token::authority = pool_authority)]
    pub pool_account_b: Account<Token>,
    #[account(mut)]
    pub payer: Signer,
    pub token_program: Program<Token>,
    pub system_program: Program<System>,
    pub rent: Sysvar<Rent>,
}

impl CreatePool {
    #[inline(always)]
    pub fn create_pool(&mut self) -> Result<(), ProgramError> {
        self.pool.amm = *self.amm.address();
        self.pool.mint_a = *self.mint_a.address();
        self.pool.mint_b = *self.mint_b.address();
        Ok(())
    }
}
