use anchor_lang::prelude::*;

use anchor_spl::token_interface::{Mint, TokenAccount, TokenInterface};

use crate::{constants::*, errors::AssetLeasingError, Lease, Listing};

/// Allows the asset owner to reclaim their asset after a lease expires
/// and the renter hasn't returned it.
///
/// Design note: In production you'd want a delegate/freeze authority
/// pattern to force-return the token. For this teaching example, the
/// owner calling claim_expired closes the lease account and frees the
/// listing. The renter should have already called return_asset.
///
/// Teaching point: enforcing asset return on-chain without freeze authority
/// requires trust or additional mechanisms (collateral, reputation, etc.)
#[derive(Accounts)]
pub struct ClaimExpiredAccountConstraints<'info> {
    #[account(mut)]
    pub owner: Signer<'info>,

    pub asset_mint: InterfaceAccount<'info, Mint>,

    /// CHECK: renter is validated via lease.renter
    pub renter: UncheckedAccount<'info>,

    #[account(
        mut,
        has_one = owner,
        has_one = asset_mint,
        seeds = [LISTING_SEED, owner.key().as_ref(), asset_mint.key().as_ref()],
        bump = listing.bump,
    )]
    pub listing: Account<'info, Listing>,

    #[account(
        mut,
        associated_token::mint = asset_mint,
        associated_token::authority = listing,
        associated_token::token_program = token_program,
    )]
    pub vault: InterfaceAccount<'info, TokenAccount>,

    /// The renter's ATA — included for completeness; in production you'd
    /// force-transfer tokens back via delegate authority
    #[account(
        mut,
        associated_token::mint = asset_mint,
        associated_token::authority = renter,
        associated_token::token_program = token_program,
    )]
    pub renter_token_account: InterfaceAccount<'info, TokenAccount>,

    #[account(
        mut,
        close = owner,
        has_one = renter,
        constraint = lease.listing == listing.key(),
        seeds = [LEASE_SEED, listing.key().as_ref(), renter.key().as_ref()],
        bump = lease.bump,
    )]
    pub lease: Account<'info, Lease>,

    pub token_program: Interface<'info, TokenInterface>,
    pub system_program: Program<'info, System>,
}

pub fn handle_claim_expired(context: Context<ClaimExpiredAccountConstraints>) -> Result<()> {
    let clock = Clock::get()?;

    require!(
        clock.unix_timestamp > context.accounts.lease.end_time,
        AssetLeasingError::LeaseNotExpired
    );

    require!(
        !context.accounts.lease.returned,
        AssetLeasingError::LeaseAlreadyReturned
    );

    // Clear the active lease flag so the asset can be delisted or re-leased
    context.accounts.listing.active_lease = false;

    // The lease account is closed via the `close = owner` constraint,
    // returning rent to the owner

    Ok(())
}
