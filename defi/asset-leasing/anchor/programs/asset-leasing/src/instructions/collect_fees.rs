use anchor_lang::prelude::*;

use crate::{constants::*, LeaseConfig};

/// Collects accumulated protocol fees.
/// Fees are paid directly to the authority in SOL during rent_asset,
/// so this instruction exists primarily as a governance mechanism to
/// update the fee rate. In a more complex system, fees might accumulate
/// in a PDA vault — here we keep it simple since SOL goes directly
/// to the authority on each rental.
#[derive(Accounts)]
pub struct CollectFeesAccountConstraints<'info> {
    pub authority: Signer<'info>,

    #[account(
        mut,
        has_one = authority,
        seeds = [LEASE_CONFIG_SEED],
        bump = lease_config.bump,
    )]
    pub lease_config: Account<'info, LeaseConfig>,
}

/// Update the protocol fee rate. Only the authority can do this.
pub fn handle_collect_fees(
    context: Context<CollectFeesAccountConstraints>,
    new_fee_basis_points: u16,
) -> Result<()> {
    require!(
        new_fee_basis_points <= MAX_FEE_BASIS_POINTS,
        crate::errors::AssetLeasingError::FeeTooHigh
    );

    context.accounts.lease_config.fee_basis_points = new_fee_basis_points;

    Ok(())
}
