use quasar_lang::prelude::*;

/// Global protocol configuration — one per deployment.
#[account(discriminator = [1])]
pub struct LeaseConfig {
    pub authority: Address,
    pub fee_basis_points: u16,
    pub bump: u8,
}

/// A listed asset available for leasing.
#[account(discriminator = [2])]
pub struct Listing {
    pub owner: Address,
    pub asset_mint: Address,
    /// SOL lamports per second of lease time
    pub price_per_second: u64,
    pub min_duration: i64,
    pub max_duration: i64,
    /// Tracks whether there's an active lease preventing delist (0=no, 1=yes)
    pub active_lease: u8,
    pub bump: u8,
}

/// An active lease — created when a renter pays to borrow an asset.
#[account(discriminator = [3])]
pub struct Lease {
    pub renter: Address,
    pub listing: Address,
    pub start_time: i64,
    pub end_time: i64,
    /// 0 = not returned, 1 = returned
    pub returned: u8,
    pub bump: u8,
}
