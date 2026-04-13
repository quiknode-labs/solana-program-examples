#![cfg_attr(not(test), no_std)]

//! Asset Leasing Program — Quasar implementation
//!
//! Note: Quasar is pre-release (0.0.0) and its API differs significantly
//! from Anchor. This implementation demonstrates the Quasar patterns.
//! Key differences:
//! - Zero-copy account access via generated Zc* types
//! - Pod types (PodU64, PodI64, etc.) for alignment-1 fields
//! - `Interface<TokenInterface>` for token program
//! - `quasar_lang::cpi::system::transfer` for SOL transfers
//! - `Clock::get()` via `quasar_lang::sysvars::Sysvar` trait
//!
//! Quasar's `#[derive(Accounts)]` does not yet support cross-field
//! references in seed expressions (e.g., `seeds = [SEED, owner.address()]`).
//! Until this is supported, PDA derivation with dynamic seeds requires
//! manual validation in the handler. This example uses the impl-block
//! pattern matching the official Quasar template.

pub mod constants;
pub mod errors;
pub mod state;

use quasar_lang::prelude::*;

declare_id!("L3aseQuasarXXXXXXXXXXXXXXXXXXXXXXXXXXXXXXXX");

/// Minimal initialize instruction to demonstrate the Quasar pattern.
/// The full instruction set mirrors the Anchor version.
#[derive(Accounts)]
pub struct InitializeAccountConstraints<'info> {
    pub authority: &'info mut Signer,

    #[account(
        init,
        payer = authority,
        space = state::LeaseConfig::SPACE,
        seeds = [constants::LEASE_CONFIG_SEED],
        bump,
    )]
    pub lease_config: &'info mut Account<state::LeaseConfig>,

    pub system_program: &'info Program<System>,
}

impl<'info> InitializeAccountConstraints<'info> {
    #[inline(always)]
    pub fn handle_initialize(&mut self, fee_basis_points: u16) -> Result<(), ProgramError> {
        require!(
            fee_basis_points <= constants::MAX_FEE_BASIS_POINTS,
            errors::AssetLeasingError::FeeTooHigh
        );

        self.lease_config.authority = *self.authority.address();
        self.lease_config.fee_basis_points = PodU16::from(fee_basis_points);

        Ok(())
    }
}

#[derive(Accounts)]
pub struct CollectFeesAccountConstraints<'info> {
    pub authority: &'info Signer,

    #[account(
        mut,
        seeds = [constants::LEASE_CONFIG_SEED],
        bump,
    )]
    pub lease_config: &'info mut Account<state::LeaseConfig>,
}

impl<'info> CollectFeesAccountConstraints<'info> {
    #[inline(always)]
    pub fn handle_collect_fees(&mut self, new_fee_basis_points: u16) -> Result<(), ProgramError> {
        require!(
            new_fee_basis_points <= constants::MAX_FEE_BASIS_POINTS,
            errors::AssetLeasingError::FeeTooHigh
        );

        require_keys_eq!(
            self.lease_config.authority,
            *self.authority.address(),
            ProgramError::IllegalOwner
        );

        self.lease_config.fee_basis_points = PodU16::from(new_fee_basis_points);

        Ok(())
    }
}

#[program]
mod asset_leasing {
    use super::*;

    #[instruction(discriminator = 0)]
    pub fn initialize(
        context: Ctx<InitializeAccountConstraints>,
        fee_basis_points: u16,
    ) -> Result<(), ProgramError> {
        context.accounts.handle_initialize(fee_basis_points)
    }

    #[instruction(discriminator = 1)]
    pub fn collect_fees(
        context: Ctx<CollectFeesAccountConstraints>,
        new_fee_basis_points: u16,
    ) -> Result<(), ProgramError> {
        context.accounts.handle_collect_fees(new_fee_basis_points)
    }

    // Instructions that require cross-field PDA seeds (list_asset,
    // delist_asset, rent_asset, return_asset, claim_expired) are
    // defined in the Anchor version. Quasar's #[derive(Accounts)]
    // does not yet support referencing other fields in seed expressions.
    //
    // When Quasar adds this capability, port the remaining instructions
    // following the same patterns shown in initialize and collect_fees.
    // See the anchor/ directory for the complete implementation.
}

#[cfg(test)]
mod tests;
