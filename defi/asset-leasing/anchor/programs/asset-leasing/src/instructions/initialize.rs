use anchor_lang::prelude::*;

use crate::{constants::*, errors::AssetLeasingError, LeaseConfig};

#[derive(Accounts)]
pub struct InitializeAccountConstraints<'info> {
    #[account(mut)]
    pub authority: Signer<'info>,

    #[account(
        init,
        payer = authority,
        space = ANCHOR_DISCRIMINATOR + LeaseConfig::INIT_SPACE,
        seeds = [LEASE_CONFIG_SEED],
        bump
    )]
    pub lease_config: Account<'info, LeaseConfig>,

    pub system_program: Program<'info, System>,
}

pub fn handle_initialize(
    context: Context<InitializeAccountConstraints>,
    fee_basis_points: u16,
) -> Result<()> {
    require!(
        fee_basis_points <= MAX_FEE_BASIS_POINTS,
        AssetLeasingError::FeeTooHigh
    );

    context.accounts.lease_config.set_inner(LeaseConfig {
        authority: context.accounts.authority.key(),
        fee_basis_points,
        bump: context.bumps.lease_config,
    });

    Ok(())
}
