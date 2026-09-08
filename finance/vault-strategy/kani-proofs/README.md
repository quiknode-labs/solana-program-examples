# Vault-strategy: Kani proofs

Formal-verification harnesses for the ERC4626-style share vault, in the spirit
of [`aeyakovenko/percolator`](https://github.com/aeyakovenko/percolator), which
uses the [Kani](https://github.com/model-checking/kani) model checker to prove
the mathematical correctness of a DeFi engine.

## What is verified

Depositors mint share tokens against the vault's net asset value; withdrawals
burn shares for a proportional slice of every vault balance; a manager fee mints
a small slice of shares over time. Token movement is via SPL CPIs Kani cannot
symbolically execute, but the share math is pure integer arithmetic:

Every share-price division carries the program's virtual offset: `VIRTUAL_SHARES` (1,000, the share mint's three extra decimals over USDC) is added to the real supply and `VIRTUAL_ASSETS` (one USDC minor unit) to the net asset value. That is the first-depositor defense, and the harnesses prove what it buys.

- `proof_withdraw_within_balance`: **Solvency**: a withdrawal never takes more of any vault balance than it holds (`floor(balance·shares/(total + VIRTUAL_SHARES)) < balance` for a positive balance, since `shares <= total`); burning the whole real supply leaves at most the virtual shares' slice behind.
- `proof_deposit_withdraw_cannot_extract`: A deposit→withdraw round-trip never returns more than was deposited, the empty vault included: no rounding attack mints shares worth more than they cost.
- `proof_first_deposit_is_priced_by_the_offset`: A deposit into an empty vault mints exactly `VIRTUAL_SHARES` share minor units per USDC minor unit (one whole nine-decimal share per USDC), with no `total_shares == 0` branch.
- `proof_donation_cannot_zero_a_deposit`: The inflation attack modelled directly (an attacker's first deposit, a donation straight into the vault, the victim's deposit): the victim's deposit mints at least one share whenever the vault holds less than `VIRTUAL_SHARES` times it, whatever the attacker did.
- `proof_fee_shares_bounded_by_supply`: The time-based manager fee, which dilutes the real supply only, can never mint more than 100%/year of dilution (`fee_shares <= total_shares` for `elapsed <= 1yr`, `fee_bps <= 10000`).

## Bounded model checking

The nonlinear harnesses verify 128-bit arithmetic with a symbolic divisor (the
share supply / NAV), so (as percolator does) they bound their symbolic inputs to
a representative range; the share identities are scale-invariant.

- `proof_withdraw_within_balance`: balances/supply `<= 255`
- `proof_deposit_withdraw_cannot_extract`: `<= 31`
- `proof_first_deposit_is_priced_by_the_offset`: unbounded (linear)
- `proof_donation_cannot_zero_a_deposit`: deposits `<= 15`, donation `<= 15,000`
- `proof_fee_shares_bounded_by_supply`: `<= 255`

Run weekly in CI (the `kani.yml` `verify` job), not on every push/PR, because
the bounded nonlinear proofs are slow. A fast unit-test job runs per push/PR.

## Running

```bash
cargo test                                                 # unit tests, no Kani
cargo install --locked kani-verifier && cargo kani setup   # one-time
cargo kani                                                  # formal verification
```
