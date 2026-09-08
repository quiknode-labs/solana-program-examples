# Changelog

## 2026-09-08

### Changed

- **Virtual shares and virtual assets.** Every share-price division now adds `VIRTUAL_SHARES` (1,000, `10^SHARE_DECIMALS_OFFSET`) to the real supply and `VIRTUAL_ASSETS` (one USDC minor unit) to the net asset value: `deposit` mints `usdc × (total_shares + 1,000) / (NAV + 1)` and `withdraw` pays `balance × shares / (total_shares + 1,000)` per vault. This is the defense against the first-depositor inflation attack (a dust deposit, a donation straight into a vault, then a deposit that floors to zero shares): a deposit now floors to zero only when the strategy already holds more than a thousand times it, and whoever inflated it that far loses about a thousand times what the depositor loses. The `total_shares == 0` special case in `deposit` is gone; `minimum_shares` and `SlippageTooHigh` stay.
- **Share mint decimals 6 → 9** (`SHARE_DECIMALS`, USDC's six plus the offset of three), so one whole share still tracks one USDC at launch: a 900 USDC first deposit mints 900,000,000,000 share minor units, which reads as 900 shares. Clients that formatted shares at six decimals must use nine.
- `collect_fees` is unchanged and dilutes the real supply only; the virtual shares earn the manager nothing.

### Added

- `test_donation_does_not_inflate_share_price` runs the attack against the live program: one minor unit deposited, 1,000 USDC transferred straight to the USDC vault, then a 1,000 USDC deposit with `minimum_shares` = 0. The victim's shares are nonzero and redeem for about 999.75 USDC; the attacker's redeem for about 500 USDC.
- The Kani proof crate models the offset and adds `proof_first_deposit_is_priced_by_the_offset` and `proof_donation_cannot_zero_a_deposit`.

## 2026-07-20

- **`WhitelistEntry` renamed `ApprovedAsset`** (and `whitelist_asset` renamed `approve_asset`, PDA seed `"whitelist"` renamed `"approved_asset"`), naming the account after what it is: one curator-approved asset bound to its official price feed. The unused `AssetNotWhitelisted` error is removed; approval is checked by the `ApprovedAsset` account's existence. Doc comments and README now state that the `Registry` account is the curator record at the root of the approved set, not the list itself.

## 2026-07-01

### Added

- **Curated asset registry.** A `Registry` plus per-mint `WhitelistEntry` accounts, maintained by a protocol authority separate from strategy managers. Each entry binds an approved mint to its official Pyth price feed. New instructions: `initialize_registry`, `whitelist_asset`.
- **Dynamic assets.** A strategy now grows its portfolio with `add_asset`, which registers a whitelisted mint at the next index as an `AssetConfig` PDA (`["asset", strategy, index]`) and creates its vault. Assets occupy the contiguous range `0..asset_count`, up to `MAX_ASSETS` (16). Replaces the previous fixed two-asset layout.
- **Oracle-bounded slippage.** `deposit` and `rebalance` compute each swap's minimum output from the Pyth price and a strategy-level `max_slippage_bps` (capped at `MAX_SLIPPAGE_BPS` = 10%), instead of trusting a caller-supplied minimum. Set at creation via `initialize_strategy`.
- **Full-allocation invariant with immediate deployment.** A strategy accepts deposits only once its weights sum to exactly 10,000 bps (`deposit` reverts with `StrategyNotFullyAllocated` otherwise). `deposit` then swaps each depositor's USDC into the basket at its target weights through the registered router in the same transaction, so every deposit is fully invested (bar sub-cent rounding dust) and the USDC vault holds no idle cash.
- **Retirable assets.** `set_weight(weight_bps)` changes an asset's target weight after creation, including setting it to zero to retire it (reassign that weight to another asset to reach 100% and reopen deposits; `rebalance` liquidates the retired holdings). The asset's index is preserved, so the `0..asset_count` range the valuation handlers depend on stays contiguous.

### Changed

- `initialize_strategy` now takes `(index, fee_bps, max_slippage_bps, swap_router)` and binds the strategy to a registry; the strategy PDA is seeded by a caller-chosen index (`["strategy", index]`) rather than the manager's key, with the manager kept as a stored field. Weights and price feeds move to `add_asset`.
- `deposit` takes each asset's `[asset_config, vault, mint, rate, price_feed]` plus the router accounts, validates the complete `0..asset_count` set for NAV, requires the strategy to be fully allocated, and deploys the deposit at the target weights.
- `withdraw` takes each asset's `[asset_config, vault, mint, user_token_account]` and pays out every asset in kind over the complete `0..asset_count` set.
- `rebalance` takes `(sell_amount, usdc_to_invest)`; per-call minimums are gone.

### Fixed

- Boxed the `mock-swap-router` swap account structs, which overflowed the 4096-byte SBF stack frame under current platform-tools.
- Documented the per-manifest build (the workspace build strips the router entrypoint via feature unification).
