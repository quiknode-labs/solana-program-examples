use anchor_lang::prelude::*;

/// Global protocol configuration — one per deployment.
/// Stores the authority who can collect fees and the fee rate.
#[account]
#[derive(InitSpace)]
pub struct LeaseConfig {
    pub authority: Pubkey,
    pub fee_basis_points: u16,
    pub bump: u8,
}

/// A listed asset available for leasing.
/// Created when an owner deposits a token into the program vault.
#[account]
#[derive(InitSpace)]
pub struct Listing {
    pub owner: Pubkey,
    pub asset_mint: Pubkey,
    /// SOL lamports per second of lease time
    pub price_per_second: u64,
    pub min_duration: i64,
    pub max_duration: i64,
    /// Tracks whether there's an active lease preventing delist
    pub active_lease: bool,
    pub bump: u8,
}

/// An active lease — created when a renter pays to borrow an asset.
#[account]
#[derive(InitSpace)]
pub struct Lease {
    pub renter: Pubkey,
    pub listing: Pubkey,
    pub start_time: i64,
    pub end_time: i64,
    pub returned: bool,
    pub bump: u8,
}
