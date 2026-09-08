//! Kani proof harnesses for the vault-strategy program (`finance/vault-strategy`).
//!
//! Inspired by aeyakovenko/percolator, which uses the Kani model checker to
//! prove the mathematical correctness of a DeFi engine's pure numeric core.
//!
//! The program is an ERC4626-style share vault: depositors mint share tokens
//! against the vault's net asset value, and withdrawals burn shares for a
//! proportional slice of every vault balance. A manager fee mints a small slice
//! of shares over time. Token movement is via SPL CPIs Kani cannot symbolically
//! execute, but the share math (`deposit`, `withdraw`, `collect_fees`) is pure
//! integer arithmetic. This crate reproduces it faithfully and proves the
//! invariants the vault's solvency rests on.
//!
//! Every share-price division carries the program's virtual offset: `VIRTUAL_SHARES`
//! (10^3, the share mint's three extra decimals over USDC) is added to the real
//! supply and `VIRTUAL_ASSETS` (one USDC minor unit) to the net asset value. That is
//! the first-depositor defense, and the harnesses prove what it buys: a deposit
//! into an empty vault mints `VIRTUAL_SHARES` share minor units per USDC minor unit,
//! and a donation into the vault cannot floor a deposit to zero shares unless the
//! vault already holds more than `VIRTUAL_SHARES` times that deposit.
//!
//! Nonlinear 128-bit harnesses use bounded model checking (small symbolic
//! inputs), as percolator does; the share identities are scale-invariant.

#![cfg_attr(kani, allow(dead_code))]

/// Virtual shares added to the real supply in every share-price division
/// (`state::VIRTUAL_SHARES` in the program): `10^SHARE_DECIMALS_OFFSET`.
pub const VIRTUAL_SHARES: u64 = 1_000;

/// Virtual assets added to the net asset value in every share-price division
/// (`state::VIRTUAL_ASSETS` in the program): one USDC minor unit.
pub const VIRTUAL_ASSETS: u64 = 1;

/// `floor((a*b)/d)`, `None` on overflow / zero divisor.
pub fn mul_div_floor(a: u128, b: u128, d: u128) -> Option<u128> {
    if d == 0 {
        return None;
    }
    a.checked_mul(b)?.checked_div(d)
}

/// Proportional withdrawal of one vault balance:
/// `floor(balance * shares / (total_shares + VIRTUAL_SHARES))` — the formula
/// `handle_withdraw` applies to the USDC leg and to every basket asset. The
/// virtual shares' slice of the balance is never paid out.
pub fn withdraw_amount(balance: u64, shares_burned: u64, total_shares: u64) -> Option<u64> {
    mul_div_floor(
        balance as u128,
        shares_burned as u128,
        total_shares as u128 + VIRTUAL_SHARES as u128,
    )?
    .try_into()
    .ok()
}

/// Shares minted for a deposit:
/// `floor(usdc_amount * (total_shares + VIRTUAL_SHARES) / (nav + VIRTUAL_ASSETS))`
/// (`handle_deposit`). There is no special case for an empty vault: with
/// `total_shares == 0` and `nav == 0` the offset alone prices the deposit at
/// `VIRTUAL_SHARES` share minor units per USDC minor unit.
pub fn deposit_shares(usdc_amount: u64, total_shares: u64, nav: u64) -> Option<u64> {
    mul_div_floor(
        usdc_amount as u128,
        total_shares as u128 + VIRTUAL_SHARES as u128,
        nav as u128 + VIRTUAL_ASSETS as u128,
    )?
    .try_into()
    .ok()
}

// ===========================================================================
// 1. Withdrawal solvency
// ===========================================================================

/// A withdrawal can never take more of any vault balance than it holds. Because
/// the burned shares are at most the real supply, and the divisor is the real
/// supply plus the virtual shares, the proportional slice
/// `floor(balance * shares / (total + VIRTUAL_SHARES))` is `< balance` whenever
/// the balance is positive. This holds for the USDC leg and for every in-kind
/// asset leg, so a withdrawal can never overdraw a vault — the core solvency
/// property — and burning the entire real supply leaves the virtual shares'
/// slice, at most `VIRTUAL_SHARES` parts in `total + VIRTUAL_SHARES`, behind.
#[cfg(kani)]
#[kani::proof]
#[kani::solver(cadical)]
fn proof_withdraw_within_balance() {
    let balance: u64 = kani::any();
    let shares_burned: u64 = kani::any();
    let total_shares: u64 = kani::any();

    // Bounded model checking (nonlinear `balance * shares`, symbolic divisor
    // `total_shares + VIRTUAL_SHARES`).
    kani::assume(balance as u128 <= 255);
    kani::assume(total_shares >= 1 && total_shares <= 255);
    kani::assume(shares_burned <= total_shares); // can't burn more than supply

    let out = withdraw_amount(balance, shares_burned, total_shares).expect("computes");
    assert!(out <= balance);
    if balance > 0 {
        // The virtual shares keep their slice: nobody can take a whole balance.
        assert!(out < balance);
    }
    // Withdrawing the entire real supply leaves at most the virtual shares' slice
    // (rounded up) behind.
    if shares_burned == total_shares {
        let kept = balance - out;
        let virtual_slice = (balance as u128 * VIRTUAL_SHARES as u128)
            / (total_shares as u128 + VIRTUAL_SHARES as u128);
        assert!(kept as u128 <= virtual_slice + 1);
    }
}

// ===========================================================================
// 2. Deposit -> withdraw round-trip cannot extract value
// ===========================================================================

/// In a USDC-only vault (NAV == vault USDC, no basket assets), depositing and
/// immediately withdrawing the minted shares never returns more USDC than was
/// deposited. Both legs floor in the protocol's favour, and the offset prices the
/// deposit against `nav + VIRTUAL_ASSETS` while the withdrawal pays from the real
/// balance, so a deposit/withdraw round-trip is never profitable — there is no
/// rounding attack that mints shares worth more than they cost. The bound covers
/// the empty vault too: `total_shares == 0`, `nav == 0`.
#[cfg(kani)]
#[kani::proof]
#[kani::solver(cadical)]
fn proof_deposit_withdraw_cannot_extract() {
    let amount: u64 = kani::any();
    let total_shares: u64 = kani::any();
    let nav: u64 = kani::any(); // == vault USDC balance for a USDC-only vault

    // Bounded model checking.
    kani::assume(amount <= 31);
    kani::assume(total_shares <= 31);
    kani::assume(nav <= 31);

    let minted = deposit_shares(amount, total_shares, nav).expect("computes");

    // State after the deposit.
    let new_total = total_shares + minted;
    let new_vault = nav + amount;

    // Withdraw exactly the freshly minted shares.
    let back = withdraw_amount(new_vault, minted, new_total).expect("computes");
    assert!(back <= amount); // round-trip never profitable
}

// ===========================================================================
// 3. The empty vault and the first deposit
// ===========================================================================

/// A deposit into an empty vault (`total_shares == 0`, `nav == 0`) mints exactly
/// `VIRTUAL_SHARES` share minor units per USDC minor unit: the offset alone
/// prices it, with no `total_shares == 0` branch. At the share mint's nine
/// decimals that is one whole share per USDC, so a 900 USDC first deposit reads
/// as 900 shares.
#[cfg(kani)]
#[kani::proof]
fn proof_first_deposit_is_priced_by_the_offset() {
    let amount: u64 = kani::any();
    kani::assume(amount as u128 * VIRTUAL_SHARES as u128 <= u64::MAX as u128);

    let minted = deposit_shares(amount, 0, 0).expect("computes");
    assert_eq!(minted, amount * VIRTUAL_SHARES);
}

// ===========================================================================
// 4. A donation cannot floor a deposit to zero
// ===========================================================================

/// The inflation attack: a dust first deposit, then a donation straight into the
/// vault (which counts toward NAV without minting shares), so that the next
/// deposit floors to zero shares. With the offset, a nonzero deposit mints at
/// least one share whenever `VIRTUAL_SHARES * amount > nav`, whatever the real
/// supply is, because `amount * (total + VIRTUAL_SHARES) >= amount * VIRTUAL_SHARES
/// > nav + VIRTUAL_ASSETS - 1`. Flooring a deposit to nothing therefore takes a
/// vault already holding more than `VIRTUAL_SHARES` times that deposit, and an
/// attacker who inflates the vault that far shares the inflation with the virtual
/// shares. The harness models the attack directly: an attacker's first deposit,
/// a donation, then the victim's deposit.
#[cfg(kani)]
#[kani::proof]
#[kani::solver(cadical)]
fn proof_donation_cannot_zero_a_deposit() {
    let attacker_deposit: u64 = kani::any();
    let donation: u64 = kani::any();
    let victim_deposit: u64 = kani::any();

    // Bounded model checking (nonlinear products with a symbolic divisor).
    kani::assume(attacker_deposit >= 1 && attacker_deposit <= 15);
    kani::assume(donation <= 15_000);
    kani::assume(victim_deposit >= 1 && victim_deposit <= 15);

    // The attacker's deposit into the empty vault.
    let attacker_shares = deposit_shares(attacker_deposit, 0, 0).expect("computes");
    // The donation lands in the vault without going through the handler.
    let nav = attacker_deposit + donation;
    // The victim's deposit.
    let victim_shares = deposit_shares(victim_deposit, attacker_shares, nav).expect("computes");

    if (nav as u128) < VIRTUAL_SHARES as u128 * victim_deposit as u128 {
        assert!(victim_shares >= 1);
    }
}

// ===========================================================================
// 5. Manager fee dilution is bounded
// ===========================================================================

/// The time-based manager fee mints
/// `fee_shares = floor(total_shares * fee_bps * elapsed / (10_000 * SECONDS_PER_YEAR))`
/// against the real supply only; the virtual shares hold nothing of anyone's and
/// earn the manager nothing. Over at most one year (`elapsed <= SECONDS_PER_YEAR`)
/// with a valid fee rate (`fee_bps <= 10_000`), the combined numerator factor
/// `fee_bps * elapsed` is `<= 10_000 * SECONDS_PER_YEAR`, so
/// `fee_shares <= total_shares`: the manager can never mint more than a
/// 100%-per-year dilution. Modelled with the combined
/// `numerator_factor <= denominator` (the constraint the two bounds imply).
#[cfg(kani)]
#[kani::proof]
#[kani::solver(cadical)]
fn proof_fee_shares_bounded_by_supply() {
    let total_shares: u64 = kani::any();
    let numerator_factor: u128 = kani::any(); // fee_bps * elapsed
    let denominator: u128 = kani::any(); // 10_000 * SECONDS_PER_YEAR

    kani::assume(total_shares as u128 <= 255);
    kani::assume(denominator >= 1 && denominator <= 255);
    // fee_bps <= 10_000 and elapsed <= SECONDS_PER_YEAR together give:
    kani::assume(numerator_factor <= denominator);

    let fee_shares =
        mul_div_floor(total_shares as u128, numerator_factor, denominator).expect("computes");
    assert!(fee_shares <= total_shares as u128); // <= 100%/year dilution
}

// ===========================================================================
// Plain unit tests.
// ===========================================================================

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn withdraw_proportional() {
        // Burn half the supply -> just under half the balance: the 1000 virtual
        // shares keep their slice.
        assert_eq!(withdraw_amount(1_000_000, 500, 1000).unwrap(), 250_000);
        // Burn all -> all but the virtual shares' slice.
        assert_eq!(withdraw_amount(1_000_000, 1000, 1000).unwrap(), 500_000);
        // At a real supply that dwarfs the offset the slice is rounding noise.
        assert_eq!(
            withdraw_amount(1_000_000, 1_000_000_000, 1_000_000_000).unwrap(),
            999_999
        );
    }

    #[test]
    fn first_deposit_reads_as_whole_shares() {
        // 900 USDC (900,000,000 minor units) into an empty vault mints
        // 900,000,000,000 share minor units: 900 whole shares at nine decimals.
        assert_eq!(deposit_shares(900_000_000, 0, 0).unwrap(), 900_000_000_000);
    }

    #[test]
    fn later_deposit_pays_the_share_price() {
        // 480 USDC at a 960 USDC NAV against 900 whole shares buys 450 whole
        // shares, the offset showing up ten digits down.
        let bob = deposit_shares(480_000_000, 900_000_000_000, 960_000_000).unwrap();
        assert_eq!(bob / 1_000_000_000, 450);
        assert_eq!(bob, 450_000_000_031);
    }

    #[test]
    fn donation_does_not_zero_a_deposit() {
        // The attack from the book: one minor unit deposited, 1,000 USDC donated,
        // then a 1,000 USDC deposit.
        let attacker = deposit_shares(1, 0, 0).unwrap();
        assert_eq!(attacker, VIRTUAL_SHARES);
        let nav = 1 + 1_000_000_000;
        let victim = deposit_shares(1_000_000_000, attacker, nav).unwrap();
        assert_eq!(victim, 1_999);
        // The victim redeems all but a fraction of a dollar of the 2,000.000001
        // USDC vault; the attacker's 1000 share minor units are worth half of the
        // rest, the virtual shares' half being nobody's.
        let vault = nav + 1_000_000_000;
        let victim_out = withdraw_amount(vault, victim, attacker + victim).unwrap();
        assert!(victim_out > 999_000_000 && victim_out <= 1_000_000_000);
        let attacker_out = withdraw_amount(vault - victim_out, attacker, attacker).unwrap();
        assert!(attacker_out < 501_000_000);
    }

    #[test]
    fn round_trip_not_profitable() {
        let minted = deposit_shares(100, 200, 150).unwrap();
        let back = withdraw_amount(150 + 100, minted, 200 + minted).unwrap();
        assert!(back <= 100);
    }
}
