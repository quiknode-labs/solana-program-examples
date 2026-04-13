pub mod constants;
pub mod errors;
pub mod instructions;
pub mod state;

use anchor_lang::prelude::*;

pub use constants::*;
pub use instructions::*;
pub use state::*;

declare_id!("9GSiyP3PBh3oH9toXJnang1gqh925qJxDm4zYcmSkUgt");

#[program]
pub mod asset_leasing {
    use super::*;

    pub fn initialize(
        context: Context<InitializeAccountConstraints>,
        fee_basis_points: u16,
    ) -> Result<()> {
        instructions::initialize::handle_initialize(context, fee_basis_points)
    }

    pub fn list_asset(
        context: Context<ListAssetAccountConstraints>,
        price_per_second: u64,
        min_duration: i64,
        max_duration: i64,
        amount: u64,
    ) -> Result<()> {
        instructions::list_asset::handle_list_asset(
            context,
            price_per_second,
            min_duration,
            max_duration,
            amount,
        )
    }

    pub fn delist_asset(context: Context<DelistAssetAccountConstraints>) -> Result<()> {
        instructions::delist_asset::handle_delist_asset(context)
    }

    pub fn rent_asset(
        context: Context<RentAssetAccountConstraints>,
        duration: i64,
    ) -> Result<()> {
        instructions::rent_asset::handle_rent_asset(context, duration)
    }

    pub fn return_asset(context: Context<ReturnAssetAccountConstraints>) -> Result<()> {
        instructions::return_asset::handle_return_asset(context)
    }

    pub fn claim_expired(context: Context<ClaimExpiredAccountConstraints>) -> Result<()> {
        instructions::claim_expired::handle_claim_expired(context)
    }

    pub fn collect_fees(
        context: Context<CollectFeesAccountConstraints>,
        new_fee_basis_points: u16,
    ) -> Result<()> {
        instructions::collect_fees::handle_collect_fees(context, new_fee_basis_points)
    }
}
