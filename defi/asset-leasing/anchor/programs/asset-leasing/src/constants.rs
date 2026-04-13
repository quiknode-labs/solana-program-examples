use anchor_lang::prelude::*;

/// Anchor 8-byte account discriminator size — used in space calculations
pub const ANCHOR_DISCRIMINATOR: usize = 8;

// Protocol fee: 2.5% expressed in basis points (1 bp = 0.01%)
#[constant]
pub const DEFAULT_FEE_BASIS_POINTS: u16 = 250;

#[constant]
pub const MAX_FEE_BASIS_POINTS: u16 = 10_000;

// PDA seeds
pub const LEASE_CONFIG_SEED: &[u8] = b"lease_config";
pub const LISTING_SEED: &[u8] = b"listing";
pub const LEASE_SEED: &[u8] = b"lease";
pub const VAULT_SEED: &[u8] = b"vault";
pub const FEE_VAULT_SEED: &[u8] = b"fee_vault";
