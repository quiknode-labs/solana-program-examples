pub mod constants;
pub mod errors;
pub mod instructions;
pub mod state;

use anchor_lang::prelude::*;

pub use constants::*;
pub use instructions::*;
pub use state::*;

declare_id!("HHKEhLk6dyzG4mK1isPyZiHcEMW4J1CRKryzyQ3JFtnF");

#[program]
pub mod asset_leasing {
    use super::*;

    /// Lessor lists a lease: deposits leased tokens into the leased vault and
    /// publishes the rental terms. The lease sits in `Listed` until a lessee
    /// takes it.
    pub fn create_lease(
        context: Context<CreateLease>,
        lease_id: u64,
        leased_amount: u64,
        required_collateral_amount: u64,
        rent_per_second: u64,
        duration_seconds: i64,
        maintenance_margin_bps: u16,
        liquidation_bounty_bps: u16,
        feed_id: [u8; 32],
    ) -> Result<()> {
        instructions::create_lease::handle_create_lease(
            context,
            lease_id,
            leased_amount,
            required_collateral_amount,
            rent_per_second,
            duration_seconds,
            maintenance_margin_bps,
            liquidation_bounty_bps,
            feed_id,
        )
    }

    /// Lessee takes the lease: posts collateral into the collateral vault and
    /// receives the leased tokens. Lease transitions to `Active`.
    pub fn take_lease(context: Context<TakeLease>) -> Result<()> {
        instructions::take_lease::handle_take_lease(context)
    }

    /// Stream rent from the collateral vault to the lessor, up to `end_ts`.
    /// Anyone may call this to keep the lease current.
    pub fn pay_rent(context: Context<PayRent>) -> Result<()> {
        instructions::pay_rent::handle_pay_rent(context)
    }

    /// Lessee adds more collateral to stay above the maintenance margin.
    pub fn top_up_collateral(context: Context<TopUpCollateral>, amount: u64) -> Result<()> {
        instructions::top_up_collateral::handle_top_up_collateral(context, amount)
    }

    /// Lessee returns the leased tokens (at or before `end_ts`). Accrued rent
    /// is settled and the remaining collateral is refunded.
    pub fn return_lease(context: Context<ReturnLease>) -> Result<()> {
        instructions::return_lease::handle_return_lease(context)
    }

    /// Keeper liquidates an undercollateralised lease using a Pyth price
    /// update. Collateral goes to the lessor, minus the keeper bounty.
    pub fn liquidate(context: Context<Liquidate>) -> Result<()> {
        instructions::liquidate::handle_liquidate(context)
    }

    /// After `end_ts`, if the lessee never returned the tokens, the lessor
    /// reclaims the collateral as compensation and closes the lease. Also
    /// used by the lessor to cancel an unrented (`Listed`) lease.
    pub fn close_expired(context: Context<CloseExpired>) -> Result<()> {
        instructions::close_expired::handle_close_expired(context)
    }
}
