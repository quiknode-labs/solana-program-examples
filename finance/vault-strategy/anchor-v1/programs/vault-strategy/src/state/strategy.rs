use anchor_lang::prelude::*;

/// Largest number of basket assets one strategy can hold. Not a storage limit
/// (each asset is its own account); the cap bounds how many accounts deposit and
/// withdraw, which must reference every asset at once, pull into a single
/// instruction: deposit uses 14 + 5*N accounts and withdraw 10 + 4*N, where N is the
/// asset count. At the cap of 16 that is 94 accounts for deposit (74 for withdraw),
/// within Solana's 128-account transaction lock limit but past the 1232-byte legacy
/// transaction size (which fits only ~3 assets), so a client depositing into a large
/// basket must send a v0 transaction with an Address Lookup Table. USDC is the base
/// currency, held separately, and does not count against this.
pub const MAX_ASSETS: u8 = 16;

/// Decimals of the share mint: USDC's six plus `SHARE_DECIMALS_OFFSET`, so one
/// whole share still tracks one USDC at launch (a 900 USDC first deposit reads
/// as 900 shares) while the supply carries three more digits than the USDC it
/// prices.
pub const SHARE_DECIMALS: u8 = 6 + SHARE_DECIMALS_OFFSET;

/// How many more decimals the share mint has than USDC. This is the vault's
/// first-depositor defense (virtual shares and virtual assets, the ERC-4626
/// "decimals offset"): every exchange-rate division adds `VIRTUAL_SHARES` to
/// the share supply and `VIRTUAL_ASSETS` to the net asset value, so an empty
/// fund already has a share price (one minor unit of USDC per `10^OFFSET` share
/// minor units), the `total_shares == 0` case needs no special branch, and a
/// donation straight into a vault is shared with shares nobody holds. An
/// attacker inflating the share price loses about `10^OFFSET` times whatever
/// the next depositor loses to rounding. Three leaves `total_shares: u64` room
/// for about eighteen billion whole shares.
pub const SHARE_DECIMALS_OFFSET: u8 = 3;

/// Virtual shares added to the real supply in every share-price division:
/// `10^SHARE_DECIMALS_OFFSET`. They are never minted, never burned, and their
/// slice of every vault is never paid out.
pub const VIRTUAL_SHARES: u64 = 10u64.pow(SHARE_DECIMALS_OFFSET as u32);

/// Virtual assets added to the net asset value in every share-price division:
/// one USDC minor unit, backing the virtual shares.
pub const VIRTUAL_ASSETS: u64 = 1;

/// One strategy (basket). Its address is a PDA seeded by a caller-chosen index,
/// e.g. seeds `"strategy" + 0`, so strategies are addressed by a simple counter
/// rather than by the manager's key. The index is stored here so every handler
/// can re-derive the PDA to sign for the vaults and share mint.
#[account]
#[derive(InitSpace)]
pub struct Strategy {
    /// Index used as the PDA seed, e.g. 0 for the first strategy.
    pub index: u64,
    pub manager: Pubkey,
    /// Registry whose curator approves assets. add_asset only accepts mints
    /// approved in this registry.
    pub registry: Pubkey,
    pub share_mint: Pubkey,
    pub usdc_mint: Pubkey,
    pub swap_router: Pubkey,
    /// Annual management fee in basis points (e.g. 100 = 1%).
    pub fee_bps: u16,
    /// Maximum tolerated deviation, in basis points, between a swap's output and
    /// the Pyth-implied amount on deposit/rebalance. Bounded by MAX_SLIPPAGE_BPS.
    pub max_slippage_bps: u16,
    pub total_shares: u64,
    pub last_fee_accrual_timestamp: i64,
    /// Assets live at PDAs indexed 0..asset_count, so callers can re-derive the
    /// complete set and no asset can be silently omitted from a NAV calculation.
    pub asset_count: u8,
    /// Running sum of every asset's target weight, kept <= 10000.
    pub total_weight_bps: u16,
    pub bump: u8,
}

/// One basket asset. Its address is a PDA seeded by the strategy and the asset's
/// index, so the full set is the contiguous range 0..asset_count: any handler
/// computing net asset value re-derives every index and refuses to proceed if an
/// asset account is missing.
#[account]
#[derive(InitSpace)]
pub struct AssetConfig {
    pub strategy: Pubkey,
    pub index: u8,
    pub mint: Pubkey,
    /// Pyth PriceUpdateV2 account, copied from the registry's ApprovedAsset at
    /// add time so the manager cannot substitute a feed they control.
    pub price_feed: Pubkey,
    /// Strategy-owned associated token account holding this asset.
    pub vault: Pubkey,
    /// Target share of the strategy's value in basis points. deposit deploys at these
    /// weights (the sum across assets must reach 10000 before deposits open), and the
    /// manager maintains them against price drift with rebalance.
    pub weight_bps: u16,
    pub bump: u8,
}

impl AssetConfig {
    /// Deserialize an AssetConfig passed via remaining_accounts to an owned value,
    /// verifying it is owned by this program and has the right discriminator.
    /// Avoids the lifetime invariance of `Account::try_from` on borrowed infos.
    pub fn load_checked(account: &AccountInfo) -> Result<AssetConfig> {
        require_keys_eq!(
            *account.owner,
            crate::ID,
            crate::error::VaultError::InvalidAssetAccount
        );
        let data = account.try_borrow_data()?;
        AssetConfig::try_deserialize(&mut &data[..])
            .map_err(|_| error!(crate::error::VaultError::InvalidAssetAccount))
    }
}
