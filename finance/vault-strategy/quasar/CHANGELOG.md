# Changelog

## [2026-09-08]

### Changed

- **Virtual shares and virtual assets.** Every share-price division adds
  `VIRTUAL_SHARES` (1,000) to the real supply and `VIRTUAL_ASSETS` (one USDC
  minor unit) to the net asset value, matching the Anchor build: `deposit`
  mints `usdc × (total_shares + 1,000) / (NAV + 1)` with no first-deposit
  special case, and `withdraw` pays `balance × shares / (total_shares + 1,000)`
  per vault. This is the defense against the first-depositor inflation attack.
- **Share mint decimals 6 → 9** (`SHARE_DECIMALS`), so a 900 USDC first deposit
  still reads as 900 shares.

### Added

- `donation_does_not_inflate_share_price`: the attack run against the program,
  with the victim's shares nonzero and redeeming for about 999.75 USDC and the
  attacker losing about half of the donation to the virtual shares.

## [2026-07-22]

### Changed

- Migrated both programs (`vault-strategy` and `mock-swap-router`) to Quasar
  0.1.0 (`0.1.0-release` branch, rev `be60fca`): Quasar.toml rewritten to the
  0.1.0 schema, `idl-build` feature and `lib` crate-type added, and tests
  rewritten from the direct QuasarSVM harness to `quasar-test`
  (`#[quasar_test]` fixtures, `crate::cpi` instruction builders — including
  `remaining_accounts` for the per-asset deposit accounts — and `Outcome`
  assertions). The two-program deposit test loads the sibling router's
  compiled `.so` at runtime via
  `test.add(Program::new(ROUTER_ID, &std::fs::read("../mock-swap-router/target/deploy/quasar_mock_swap_router.so")...))`,
  so `quasar build` must run in `mock-swap-router` before the vault-strategy
  tests execute. The `quasar-svm` git dev-dependency is gone; compute-unit
  assertions were dropped pending recalibration under 0.1.0. Program-source
  fix for 0.1.0 in both programs: `Seed` is no longer in the prelude, so the
  instruction files that build signer seeds now import it from
  `quasar_lang::cpi`.

## 2026-07-20

- **`WhitelistEntry` renamed `ApprovedAsset`** (and `whitelist_asset` renamed `approve_asset`, PDA seed `"whitelist"` renamed `"approved_asset"`), naming the account after what it is: one curator-approved asset bound to its official price feed. The unused `AssetNotWhitelisted` error is removed; approval is checked by the `ApprovedAsset` account's existence. Doc comments and README now state that the `Registry` account is the curator record at the root of the approved set, not the list itself.

## 2026-07-07

Added this changelog. Changes prior to this date were tracked in git history only.
