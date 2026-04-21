#![cfg_attr(not(test), no_std)]

use quasar_lang::prelude::*;

mod constants;
mod errors;
mod instructions;
mod state;

use instructions::*;
#[cfg(test)]
mod tests;

// Same program id as the Anchor version so off-chain tooling that derives
// PDAs or looks up the program on-chain works against both binaries
// interchangeably.
declare_id!("Lease11111111111111111111111111111111111111");

/// Asset-leasing program: fixed-term token leases with a streaming rent
/// payment, collateral escrow, and Pyth-oracle-triggered liquidation.
///
/// See the top-level `defi/asset-leasing/anchor/README.md` for the full
/// mechanics — the Quasar and Anchor versions are functionally identical.
#[program]
mod quasar_asset_leasing {
    use super::*;

    /// Discriminators are packed densely starting from 0 so the wire format
    /// stays a single byte. The order matches the natural user-facing
    /// lifecycle (create → take → pay/top-up → return/liquidate/close).
    #[instruction(discriminator = 0)]
    pub fn create_lease(
        ctx: Ctx<CreateLease>,
        lease_id: u64,
        leased_amount: u64,
        required_collateral_amount: u64,
        rent_per_second: u64,
        duration_seconds: i64,
        maintenance_margin_bps: u16,
        liquidation_bounty_bps: u16,
        feed_id: [u8; 32],
    ) -> Result<(), ProgramError> {
        instructions::handle_create_lease(
            &mut ctx.accounts,
            lease_id,
            leased_amount,
            required_collateral_amount,
            rent_per_second,
            duration_seconds,
            maintenance_margin_bps,
            liquidation_bounty_bps,
            feed_id,
            &ctx.bumps,
        )
    }

    #[instruction(discriminator = 1)]
    pub fn take_lease(ctx: Ctx<TakeLease>) -> Result<(), ProgramError> {
        instructions::handle_take_lease(&mut ctx.accounts)
    }

    #[instruction(discriminator = 2)]
    pub fn pay_rent(ctx: Ctx<PayRent>) -> Result<(), ProgramError> {
        instructions::handle_pay_rent(&mut ctx.accounts)
    }

    #[instruction(discriminator = 3)]
    pub fn top_up_collateral(
        ctx: Ctx<TopUpCollateral>,
        amount: u64,
    ) -> Result<(), ProgramError> {
        instructions::handle_top_up_collateral(&mut ctx.accounts, amount)
    }

    #[instruction(discriminator = 4)]
    pub fn return_lease(ctx: Ctx<ReturnLease>) -> Result<(), ProgramError> {
        instructions::handle_return_lease(&mut ctx.accounts)
    }

    #[instruction(discriminator = 5)]
    pub fn liquidate(ctx: Ctx<Liquidate>) -> Result<(), ProgramError> {
        instructions::handle_liquidate(&mut ctx.accounts)
    }

    #[instruction(discriminator = 6)]
    pub fn close_expired(ctx: Ctx<CloseExpired>) -> Result<(), ProgramError> {
        instructions::handle_close_expired(&mut ctx.accounts)
    }
}
