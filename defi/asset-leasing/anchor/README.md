# Asset Leasing

**Onchain securities lending.** Long holders rent out fungible token
inventory to short sellers. Borrowers post collateral, pay a
second-by-second lending fee, and return equivalent tokens before
expiry. If the borrowed asset rallies past the maintenance margin,
keepers liquidate the position; if it falls, the borrower profits and
returns equivalent tokens cheaply.

This is the same primitive that underpins traditional securities
lending: long inventory holders (exchange-traded funds and pension
funds in traditional finance; passive holders onchain) earn yield on
assets they would hold anyway, and short sellers and arbitrageurs get
the borrow they need. The program is written in Anchor; a parallel
[Quasar port](#7-quasar-port) implements the same onchain behaviour.

The code uses `lessor` / `lessee` identifiers throughout — those names
predate the framing change and stay as-is so the source is grep-able.
The README freely uses **lender** for the lessor and **borrower** (or
**short seller**) for the lessee; they refer to the same onchain
roles.

Every instruction handler is walked through with the exact token
movements it causes. If you already know what collateral, a
maintenance margin and an oracle are, you can skip straight to
[Accounts and program-derived addresses](#2-accounts-and-program-derived-addresses) or
[Instruction handler lifecycle walkthrough](#3-instruction-handler-lifecycle-walkthrough).

Solana terminology is defined at https://solana.com/docs/terminology.
Terms specific to this program are explained inline when they first
appear.

---

## Table of contents

1. [What does this program do?](#1-what-does-this-program-do)
2. [Accounts and program-derived addresses](#2-accounts-and-program-derived-addresses)
3. [Instruction handler lifecycle walkthrough](#3-instruction-handler-lifecycle-walkthrough)
4. [Full-lifecycle worked examples](#4-full-lifecycle-worked-examples)
5. [Safety and edge cases](#5-safety-and-edge-cases)
6. [Running the tests](#6-running-the-tests)
7. [Quasar port](#7-quasar-port)
8. [Extending the program](#8-extending-the-program)

---

## 1. What does this program do?

A **lessor (lender)** offers some quantity of one fungible token —
mint **A**, the "leased mint" — for a fixed term. A **lessee
(borrower / short seller)** posts collateral in a different mint
**B** — the "collateral mint" — to take delivery. The borrower will
typically sell the A tokens immediately on a market like Jupiter, then
re-acquire equivalent A tokens later to close out. Because mint A is
fungible, the borrower only has to return the same *quantity*, not the
exact units they received.

The program acts as a non-custodial escrow. It:

1. Takes the lender's A tokens and locks them in a program-owned vault
   until a borrower shows up.
2. When a borrower calls `take_lease`, the program locks the
   borrower's B tokens as collateral and hands the A tokens to the
   borrower.
3. While the loan is live, a second-by-second **lending fee stream**
   pays the lender out of the collateral vault.
4. If the price of A (measured in B) rises far enough that the locked
   collateral is no longer enough to cover the cost of re-acquiring
   the borrowed tokens, anyone can call `liquidate` — the collateral
   is seized, most of it goes to the lender, and a small percentage
   (the **liquidation bounty**) goes to whoever called it. Such a
   caller is known as a **keeper** — a bot or anyone else who watches
   the chain for positions that have gone underwater and earns the
   bounty by cleaning them up.
5. If the borrower returns the full A amount before the deadline, they
   get back whatever collateral is left after lending fees.
6. If the borrower ghosts past the deadline without returning
   anything, the lender calls `close_expired` and sweeps the
   collateral as compensation.

The trigger for step 4 is the **maintenance margin**: a ratio,
expressed in basis points (1 basis point = 1/100 of a percent), of
required collateral value to debt value.
`maintenance_margin_basis_points = 12_000` is 120%, meaning the
collateral must stay worth at least 1.2× the borrowed tokens. Drop
below and the position becomes liquidatable.

The program is a pair of vaults, a small piece of state that tracks
how much has been paid, and an oracle check.

### Roles

- **Lessor / lender.** Long the asset, willing to part with it
  temporarily to earn the lending fee. The economic match for this
  role is a passive holder — someone who would hold the asset anyway
  and is happy to earn yield on idle inventory.
- **Lessee / borrower / short seller.** Pays the lending fee for the
  right to sell the borrowed tokens now and buy them back later. The
  payoff shape is the same as a short: profit if the borrowed asset
  falls, loss (and possible liquidation) if it rises.
- **Keeper / liquidator.** Standard role — watches for
  undercollateralised positions and takes the bounty for closing them.

### Worked example: shorting xNVDA via the lending market

Concrete numbers using assets that already trade on Solana —
[xNVDA](https://www.backed.fi/) (a Backed Finance / xStocks tokenised
NVIDIA share) and USDC. xNVDA has its own Pyth feed; the program
takes the feed id verbatim at `create_lease`.

Alice holds 100 xNVDA at ~$180 / share, ~$18 000 notional. She wants
yield on inventory she would hold anyway.

Bob wants short exposure to NVIDIA without using a perpetual future.

Alice lists the lease (assume USDC is 6-decimal, xNVDA is also
6-decimal for round numbers):

| Parameter | Value | Notes |
|---|---|---|
| `leased_amount` | `100_000_000` (100 xNVDA) | |
| `required_collateral_amount` | `22_000_000_000` (22 000 USDC) | ~122% LTV at the spot price |
| `lease_fee_per_second` | `456` (USDC base units / s) | ≈ 8% APR on 18 000 USDC notional |
| `duration_seconds` | `2_592_000` | 30 days |
| `maintenance_margin_basis_points` | `11_000` | 110% |
| `liquidation_bounty_basis_points` | `100` | 1% of post-fee collateral |
| `feed_id` | Pyth xNVDA/USD feed id | ([Pyth feed registry](https://www.pyth.network/price-feeds)) |

Bob calls `take_lease`, posts 22 000 USDC, receives 100 xNVDA, and
sells them on Jupiter for ~18 000 USDC at the spot price.

#### If NVIDIA rallies to $200

- Bob's debt to repurchase the 100 xNVDA is now `100 × $200 = $20 000`.
- Collateral ratio: `22 000 / 20 000 = 110%` — exactly at the
  maintenance margin.
- One more upward tick and a keeper can call `liquidate` with a fresh
  Pyth update. Of the 22 000 USDC vault: a small portion has
  already streamed out as lease fees (Bob's incentive to keep paying
  was to keep the position alive); of what's left, 1% goes to the
  keeper as the bounty (~220 USDC), the rest to Alice.
- Bob can avoid this by:
  - Calling `top_up_collateral` to push the ratio back above 110%, or
  - Buying 100 xNVDA on the open market and calling `return_lease` to
    close out cleanly.

#### If NVIDIA falls to $160

- Bob's debt drops to `100 × $160 = $16 000`.
- Collateral ratio: `22 000 / 16 000 = 137.5%` — well above the 110%
  maintenance margin. No liquidation pressure.
- Bob buys back 100 xNVDA on Jupiter for ~16 000 USDC and calls
  `return_lease`. Alice receives the 100 xNVDA back plus the
  accrued lease fee. The remaining ~22 000 USDC (minus fees paid)
  refunds to Bob.
- Bob's profit ≈ `$18 000 − $16 000 − fees − trading costs ≈ $2 000`
  minus carry. This is a 30-day short on NVIDIA, expressed onchain.

The asymmetry: liquidation only ever fires when the *borrowed* asset
rallies against the collateral. A drop in the borrowed asset price is
purely beneficial to the borrower. The streaming lending fee is the
position's only ongoing cost in either direction.

§4 walks the onchain token flows for each path with abstract numbers
that match the LiteSVM tests; the example above is the same machinery
applied to a real asset pair.

### Production deviations to know

- **Pyth integration is hand-rolled, not via the SDK.** The LiteSVM
  tests install a `PriceUpdateV2` account whose layout is decoded
  inline in `liquidate.rs`. Production code would depend on the
  `pyth-solana-receiver-sdk` crate so layout changes are caught at
  compile time.
- See §5 for the rest of the deliberate simplifications.

---

## 2. Accounts and program-derived addresses

Every call to the program touches some subset of these accounts. The
three program-derived addresses are created on `create_lease` and destroyed on `return_lease`
/ `liquidate` / `close_expired`.

### State / data accounts

| Account | program-derived address? | Seeds | Kind | Authority | Holds |
|---|---|---|---|---|---|
| `Lease` | yes | `["lease", lessor, lease_id]` | data | program | all the lease parameters and current lifecycle state (see below) |

### Token vaults

| Account | program-derived address? | Seeds | Kind | Authority | Holds |
|---|---|---|---|---|---|
| `leased_vault` | yes | `["leased_vault", lease]` | token account | itself (program-derived address-signed) | `leased_amount` while `Listed`; 0 while `Active` (lessee has the tokens); full amount again briefly inside `return_lease` |
| `collateral_vault` | yes | `["collateral_vault", lease]` | token account | itself (program-derived address-signed) | 0 while `Listed`; `collateral_amount` while `Active`, decreasing as lease fee streams out and increasing on `top_up_collateral` |

### User accounts passed in

| Account | Owner | Purpose |
|---|---|---|
| `lessor` wallet | user | `create_lease` signer, receives the lease fee and final recovery |
| `lessee` wallet | user | `take_lease` / `top_up_collateral` / `return_lease` signer |
| `keeper` wallet | user | `liquidate` signer, receives the bounty |
| `payer` wallet | user | `pay_lease_fee` signer (can be anyone, not just the lessee) |
| `lessor_leased_account` | token account | lessor's associated token account for the leased mint; source on `create_lease`, destination on `return_lease` / `close_expired` |
| `lessor_collateral_account` | token account | lessor's associated token account for the collateral mint; destination for the lease fee and liquidation proceeds |
| `lessee_leased_account` | token account | lessee's associated token account for the leased mint; destination on `take_lease`, source on `return_lease` |
| `lessee_collateral_account` | token account | lessee's associated token account for the collateral mint; source on `take_lease` / `top_up_collateral`, destination for collateral refund on `return_lease` |
| `keeper_collateral_account` | token account | keeper's associated token account for the collateral mint; receives the liquidation bounty |
| `price_update` | Pyth Receiver program | `PriceUpdateV2` account for the feed the lease is pinned to |

### Fields on `Lease`

From [`state/lease.rs`](programs/asset-leasing/src/state/lease.rs):

```rust
pub struct Lease {
    pub lease_id: u64,             // caller-supplied id so one lessor can run many leases
    pub lessor: Pubkey,            // who listed it, gets paid the lease fee
    pub lessee: Pubkey,            // who took it; Pubkey::default() while Listed

    pub leased_mint: Pubkey,
    pub leased_amount: u64,        // locked at creation, unchanging

    pub collateral_mint: Pubkey,
    pub collateral_amount: u64,    // increases on top_up, decreases as lease fees pay out
    pub required_collateral_amount: u64, // what the lessee must post on take_lease

    pub lease_fee_per_second: u64,      // denominated in collateral units
    pub duration_seconds: i64,
    pub start_timestamp: i64,             // 0 while Listed
    pub end_timestamp: i64,               // 0 while Listed; start_timestamp + duration once Active
    pub last_paid_timestamp: i64,    // Lease fee accrues from here to min(now, end_timestamp)

    pub maintenance_margin_basis_points: u16,   // e.g. 12_000 = 120%
    pub liquidation_bounty_basis_points: u16,   // e.g. 500 = 5%

    pub feed_id: [u8; 32],         // Pyth feed_id this lease is pinned to

    pub status: LeaseStatus,       // Listed | Active | Liquidated | Closed

    pub bump: u8,
    pub leased_vault_bump: u8,
    pub collateral_vault_bump: u8,
}
```

### Lifecycle diagram

```
                  create_lease
               +---------------+
 (no lease) -> |    Listed     |
               +---------------+
                 |          |
      take_lease |          | close_expired (lessor cancels)
                 v          v
               +---------------+       +--------+
               |    Active     | ----> | Closed |
               +---------------+       +--------+
                 |    |       |
     return_lease|    |       | close_expired (after end_timestamp)
                 |    | liquidate
                 v    v       v
             +--------+ +-----------+
             | Closed | | Liquidated|
             +--------+ +-----------+
```

The `Closed` and `Liquidated` states are not directly observable
onchain: all three of `return_lease`, `liquidate` and `close_expired`
close the `Lease` account in the same transaction (`close = lessor`),
returning the rent-exempt lamports to the lessor. The in-memory
`status` field is set *before* the close so the transaction logs
record the terminal state, but the account disappears at the end.

---

## 3. Instruction handler lifecycle walkthrough

An *instruction* on Solana is the input sent in a transaction — a
program id, a list of accounts, and a byte payload. The Rust function
that runs when one arrives is the *instruction handler*. This program
has seven instruction handlers. The natural order a user encounters
them — the order below — is:

1. `create_lease` (lessor)
2. `take_lease` (lessee)
3. `pay_lease_fee` (anyone)
4. `top_up_collateral` (lessee)
5. `return_lease` (lessee) — **happy path**
6. `liquidate` (keeper) — **adversarial path**
7. `close_expired` (lessor) — **default / cancel path**

For each, the shape is the same: who signs, what accounts go in, which
program-derived addresses get created or closed, which tokens move, what state changes, what
checks the program runs.

Token-flow diagrams use the following shorthand:

```
  <source account> --[amount of <mint>]--> <destination account>
```

### 3.1 `create_lease`

**Who calls it:** the lessor. They want to offer some number of leased
tokens for a fixed term against collateral of a different mint.

**Signers:** `lessor`.

**Parameters:**

```rust
pub fn create_lease(
    context: Context<CreateLease>,
    lease_id: u64,
    leased_amount: u64,
    required_collateral_amount: u64,
    lease_fee_per_second: u64,
    duration_seconds: i64,
    maintenance_margin_basis_points: u16,
    liquidation_bounty_basis_points: u16,
    feed_id: [u8; 32],
) -> Result<()>
```

**Accounts in:**

- `lessor` (signer, mut — pays account rent)
- `leased_mint`, `collateral_mint` (read-only)
- `lessor_leased_account` (mut, lessor's associated token account for the leased mint — source)
- `lease` (program-derived address, **init**) — created here
- `leased_vault` (program-derived address, **init**, token account) — created here
- `collateral_vault` (program-derived address, **init**, token account) — created here
- `token_program`, `system_program`

**program-derived addresses created:**

- `lease` with seeds `[b"lease", lessor, lease_id.to_le_bytes()]`
- `leased_vault` with seeds `[b"leased_vault", lease]`, authority = itself
- `collateral_vault` with seeds `[b"collateral_vault", lease]`, authority = itself

**Checks (from `handle_create_lease`):**

- `leased_mint != collateral_mint` → `LeasedMintEqualsCollateralMint`
- `leased_amount > 0` → `InvalidLeasedAmount`
- `required_collateral_amount > 0` → `InvalidCollateralAmount`
- `lease_fee_per_second > 0` → `InvalidLeaseFeePerSecond`
- `duration_seconds > 0` → `InvalidDuration`
- `0 < maintenance_margin_basis_points <= 50_000` → `InvalidMaintenanceMargin`
- `liquidation_bounty_basis_points <= 2_000` → `InvalidLiquidationBounty`

**Token movements:**

```
  lessor_leased_account --[leased_amount of leased_mint]--> leased_vault program-derived address
```

**State changes:**

- New `Lease` account written with `status = Listed`, `lessee =
  Pubkey::default()`, `collateral_amount = 0`, `start_timestamp = 0`,
  `end_timestamp = 0`, `last_paid_timestamp = 0`, and the given parameters
  including `feed_id`. All three bumps stored.

**Why lock the leased tokens up-front rather than on `take_lease`?** So a
lessee who calls `take_lease` cannot possibly fail because the lessor
doesn't have the tokens any more — the atomicity guarantee is
transferred to the program-derived address the moment the lease is listed.

### 3.2 `take_lease`

**Who calls it:** the lessee. They have seen the `Lease` account on
chain (somehow — an indexer, a direct lookup, whatever) and want to
take delivery.

**Signers:** `lessee`.

**Accounts in:**

- `lessee` (signer, mut)
- `lessor` (UncheckedAccount — read for program-derived address seed derivation only, no
  signature required)
- `lease` (mut, `has_one = lessor`, `has_one = leased_mint`,
  `has_one = collateral_mint`, must be `Listed`)
- `leased_mint`, `collateral_mint`
- `leased_vault`, `collateral_vault` (both mut, both program-derived address-derived)
- `lessee_collateral_account` (mut, lessee's associated token account — source)
- `lessee_leased_account` (mut, **init_if_needed** — destination)
- `token_program`, `associated_token_program`, `system_program`

**Checks:**

- `lease.status == Listed` → `InvalidLeaseStatus`
- `lease.lessor == lessor.key()` (Anchor `has_one`)
- `lease.leased_mint == leased_mint.key()` (Anchor `has_one`)
- `lease.collateral_mint == collateral_mint.key()` (Anchor `has_one`)

**Token movements (in order):**

```
  lessee_collateral_account --[required_collateral_amount of collateral_mint]--> collateral_vault program-derived address
  leased_vault program-derived address         --[leased_amount of leased_mint]-----------------> lessee_leased_account
```

Collateral is deposited *first* so if the leased-token transfer fails
for any reason the whole transaction reverts and the lessee gets their
collateral back.

**State changes:**

- `lease.lessee = lessee.key()`
- `lease.collateral_amount = required_collateral_amount`
- `lease.start_timestamp = now`
- `lease.end_timestamp = now + duration_seconds` (checked add, errors on overflow)
- `lease.last_paid_timestamp = now` (nothing has accrued yet)
- `lease.status = Active`

### 3.3 `pay_lease_fee`

**Who calls it:** anyone. The lessee's incentive is obvious (keep the
lease from going underwater); a keeper bot may also push a lease fee payment before a
liquidation check so healthy leases stay healthy.

**Signers:** `payer` (any signer).

**Accounts in:**

- `payer` (signer, mut — pays for `init_if_needed` of the lessor associated token account)
- `lessor` (UncheckedAccount, read-only — used for `has_one` check)
- `lease` (mut, must be `Active`)
- `collateral_mint`, `collateral_vault`
- `lessor_collateral_account` (mut, **init_if_needed**)
- `token_program`, `associated_token_program`, `system_program`

**Lease fee math:**

```rust
pub fn compute_lease_fee_due(lease: &Lease, now: i64) -> Result<u64> {
    let cutoff = now.min(lease.end_timestamp);
    if cutoff <= lease.last_paid_timestamp {
        return Ok(0);
    }
    let elapsed = (cutoff - lease.last_paid_timestamp) as u64;
    elapsed.checked_mul(lease.lease_fee_per_second)
        .ok_or(AssetLeasingError::MathOverflow.into())
}
```

Lease fees do not accrue past `end_timestamp`. Past the deadline the lessee is
either returning the tokens (via `return_lease`), being liquidated, or
defaulting — no more lease fees are owed.

**Token movements:**

```
  collateral_vault program-derived address --[min(lease_fee_due, collateral_amount) of collateral_mint]--> lessor_collateral_account
```

If the vault does not have enough collateral to cover the full
`lease_fee_due`, the handler pays out whatever is there and leaves the
residual as a debt the next liquidation (or `close_expired`) will
clean up.

**State changes:**

- `lease.collateral_amount -= payable`
- `lease.last_paid_timestamp = now.min(end_timestamp)`

### 3.4 `top_up_collateral`

**Who calls it:** the lessee — to defend against a looming liquidation
by adding more of the collateral mint to the vault.

**Signers:** `lessee`.

**Accounts in:**

- `lessee` (signer)
- `lessor` (UncheckedAccount, read-only)
- `lease` (mut, `has_one = lessor`, `has_one = collateral_mint`,
  `constraint lease.lessee == lessee.key()`, must be `Active`)
- `collateral_mint`, `collateral_vault`
- `lessee_collateral_account` (mut, source)
- `token_program`

**Parameter:** `amount: u64` — how much to add.

**Checks:**

- `amount > 0` → `InvalidCollateralAmount`
- `lease.lessee == lessee.key()` → `Unauthorised`
- `lease.status == Active` → `InvalidLeaseStatus`

**Token movements:**

```
  lessee_collateral_account --[amount of collateral_mint]--> collateral_vault program-derived address
```

**State changes:**

- `lease.collateral_amount += amount` (checked add)

### 3.5 `return_lease`

**Who calls it:** the lessee, while the lease is still `Active` and
before or after `end_timestamp` (the only timing rule is that `status ==
Active`; Lease fees only accrue up to `end_timestamp` so returning after the
deadline does not pile on extra charges).

**Signers:** `lessee`.

**Accounts in:**

- `lessee` (signer, mut)
- `lessor` (UncheckedAccount, mut — receives Lease and vault rent-exempt
  lamports via `close = lessor`)
- `lease` (mut, `close = lessor`, must be `Active`, `lessee == lessee.key()`)
- `leased_mint`, `collateral_mint`
- `leased_vault`, `collateral_vault` (both mut)
- `lessee_leased_account` (mut, source for the return)
- `lessee_collateral_account` (mut, destination for the refund)
- `lessor_leased_account` (mut, **init_if_needed**)
- `lessor_collateral_account` (mut, **init_if_needed**)
- `token_program`, `associated_token_program`, `system_program`

**Checks:**

- `lease.status == Active` → `InvalidLeaseStatus`
- `lease.lessee == lessee.key()` → `Unauthorised`

**Token movements (in order):**

```
  lessee_leased_account   --[leased_amount of leased_mint]----------> leased_vault program-derived address
  leased_vault program-derived address        --[leased_amount of leased_mint]----------> lessor_leased_account
  collateral_vault program-derived address    --[lease_fee_payable of collateral_mint]-------> lessor_collateral_account
  collateral_vault program-derived address    --[collateral_after_lease_fees of collateral_mint]--> lessee_collateral_account
```

The leased tokens hop through the vault rather than going direct
lessee→lessor because the vault's token account is already set up and
the program can reuse its program-derived address signing path. The atomic round-trip keeps
the vault's post-instruction balance at 0 so it can be closed.

After the transfers:

- Both vaults are closed via `close_account` cross-program invocations; their rent-exempt
  lamports go to the lessor.
- The `Lease` account is closed via Anchor's `close = lessor`
  constraint; its rent-exempt lamports go to the lessor too.

**State changes before close:**

- `lease.last_paid_timestamp = now.min(end_timestamp)`
- `lease.collateral_amount = 0`
- `lease.status = Closed`

### 3.6 `liquidate`

**Who calls it:** a keeper, when they can prove the position is
underwater.

**Signers:** `keeper`.

**Accounts in:**

- `keeper` (signer, mut — pays `init_if_needed` cost for both associated token accounts)
- `lessor` (UncheckedAccount, mut — receives the lease fee + lessor_share + the
  `Lease` and vault rent-exempt lamports)
- `lease` (mut, `close = lessor`, must be `Active`)
- `leased_mint`, `collateral_mint`
- `leased_vault`, `collateral_vault` (both mut)
- `lessor_collateral_account` (mut, **init_if_needed**)
- `keeper_collateral_account` (mut, **init_if_needed**)
- `price_update` (UncheckedAccount, constrained to `owner =
  PYTH_RECEIVER_PROGRAM_ID`)
- `token_program`, `associated_token_program`, `system_program`

**Checks (in order, early-out on failure):**

1. `price_update.owner == Pyth Receiver program id` (Anchor `owner =`)
2. Account data decodes as `PriceUpdateV2` (first 8 bytes match
   `PRICE_UPDATE_V2_DISCRIMINATOR`; length ≥ 89 bytes) — else
   `StalePrice`
3. `decoded.feed_id == lease.feed_id` → `PriceFeedMismatch`
4. `publish_time <= now` (no future stamps) and
   `now - publish_time <= 60 seconds` → `StalePrice`
5. `price > 0` → `NonPositivePrice`
6. `is_underwater(lease, price, now) == true` → `PositionHealthy`
7. `lease.status == Active` (Anchor constraint on the `lease` field)

The underwater check, in integers:

```
  collateral_value_in_colla_units * 10_000
      <  debt_value_in_colla_units * maintenance_margin_basis_points
```

where `debt_value = leased_amount * price * 10^exponent` (with the
exponent folded into whichever side keeps the math non-negative, see
[`is_underwater`](programs/asset-leasing/src/instructions/liquidate.rs)).

**Token movements:**

```
  collateral_vault program-derived address --[lease_fee_payable of collateral_mint]---------------------> lessor_collateral_account
  collateral_vault program-derived address --[bounty = remaining * bounty_basis_points / 10_000]-----------> keeper_collateral_account
  collateral_vault program-derived address --[remaining - bounty of collateral_mint]--------------> lessor_collateral_account
  leased_vault program-derived address    --[0 of leased_mint]  (empty — lessee kept the tokens)    close only
```

After the three outbound collateral transfers (lease fee, bounty, lessor
share) the collateral_vault is empty. Both vaults are then closed —
their rent-exempt lamports go to the lessor. The `Lease` account is
closed the same way (Anchor `close = lessor`).

**State changes before close:**

- `lease.collateral_amount = 0`
- `lease.last_paid_timestamp = now.min(end_timestamp)`
- `lease.status = Liquidated`

### 3.7 `close_expired`

**Who calls it:** the lessor. Two very different situations collapse
into this single handler:

- **Cancel a `Listed` lease** — the lessor changes their mind, no-one
  has taken the lease yet. Allowed any time.
- **Reclaim collateral after default** — the lease is `Active`, `now >=
  end_timestamp`, the lessee has not called `return_lease`. The lessor takes
  the whole collateral vault as compensation.

**Signers:** `lessor`.

**Accounts in:**

- `lessor` (signer, mut — also the rent destination for all three closes)
- `lease` (mut, `close = lessor`, status ∈ `{Listed, Active}`)
- `leased_mint`, `collateral_mint`
- `leased_vault`, `collateral_vault` (both mut)
- `lessor_leased_account` (mut, **init_if_needed**)
- `lessor_collateral_account` (mut, **init_if_needed**)
- `token_program`, `associated_token_program`, `system_program`

**Checks:**

- `status ∈ {Listed, Active}` (Anchor `constraint matches!(...)`) →
  `InvalidLeaseStatus`
- If `status == Active`, also `now >= end_timestamp` → `LeaseNotExpired`

**Token movements:**

For a `Listed` cancel:
```
  leased_vault program-derived address --[leased_amount of leased_mint]--> lessor_leased_account
  collateral_vault program-derived address is empty (0 transferred)
```

For an `Active` default:
```
  leased_vault program-derived address is empty (lessee kept the tokens)
  collateral_vault program-derived address --[collateral_amount of collateral_mint]--> lessor_collateral_account
```

In both cases both vaults are then closed and the `Lease` account is
closed; all three rent-exempt lamport refunds go to the lessor.

**State changes before close:**

- If `Active`: `lease.last_paid_timestamp = now.min(end_timestamp)`
  (settles the accounting so any future program version that wants
  to split the default pot differently has a correct timestamp to
  start from)
- `lease.collateral_amount = 0`
- `lease.status = Closed`

---

## 4. Full-lifecycle worked examples

These are abstract walkthroughs of the same machinery the §1 xNVDA
example uses, with round numbers chosen to make the arithmetic easy
to follow and to match the LiteSVM tests one-to-one. All paths share
the same starting parameters. Both mints are 6-decimal tokens, so
1 token = 1 000 000 base units. Throughout this section, "leased
units" means base units of the leased mint and "collateral units"
means base units of the collateral mint — they are descriptive
labels, not real tickers.
The diagrams use the same convention: `[<number> leased]` and
`[<number> collateral]`.

- `leased_amount = 100_000_000` (100 leased tokens).
- `required_collateral_amount = 200_000_000` (200 collateral tokens).
- `lease_fee_per_second = 10` collateral units.
- `duration_seconds = 86_400` (24 hours).
- `maintenance_margin_basis_points = 12_000` (120%).
- `liquidation_bounty_basis_points = 500` (5% of post-lease-fee collateral).
- `feed_id = [0xAB; 32]` (arbitrary, consistent across all calls).

Lessor starts with 1 000 000 000 leased units in
their associated token account. Lessee starts with 1 000 000 000
collateral units in theirs.

### 4.1 Happy path — lessee returns on time

Calls, in order:

1. **`create_lease`** — lessor posts 100 leased tokens into
   `leased_vault`, parameters written to `lease`.
   ```
   lessor_leased_account --[100_000_000 leased]--> leased_vault program-derived address
   ```
   Balances after: lessor has 900 000 000 leased units, `leased_vault` has
   100 000 000 leased units, `collateral_vault` has 0.

2. **`take_lease`** — lessee posts 200 collateral tokens, receives
   100 leased tokens.
   ```
   lessee_collateral_account --[200_000_000 collateral]--> collateral_vault program-derived address
   leased_vault program-derived address          --[100_000_000 leased]--> lessee_leased_account
   ```
   `lease.status = Active`, `start_timestamp = T`, `end_timestamp = T + 86_400`.

3. **`pay_lease_fee`** called at `T + 120` seconds. Lease fee due = 120 × 10 =
   1 200 collateral units.
   ```
   collateral_vault program-derived address --[1_200 collateral]--> lessor_collateral_account
   ```
   `collateral_amount = 200_000_000 − 1_200 = 199_998_800`.

4. **`top_up_collateral(amount = 50_000_000)`** at `T + 600`. Lessee
   decides to add a cushion.
   ```
   lessee_collateral_account --[50_000_000 collateral]--> collateral_vault program-derived address
   ```
   `collateral_amount = 199_998_800 + 50_000_000 = 249_998_800`.

5. **`return_lease`** called at `T + 3_600` (one hour in). Total lease fees
   from `start_timestamp` to `now` is 3 600 × 10 = 36 000 collateral units; 1 200 of that
   was paid in step 3. Residual lease fees = 36 000 − 1 200 = 34 800 collateral units.
   ```
   lessee_leased_account  --[100_000_000 leased]--> leased_vault program-derived address
   leased_vault program-derived address       --[100_000_000 leased]--> lessor_leased_account
   collateral_vault program-derived address   --[34_800 collateral]--------> lessor_collateral_account
   collateral_vault program-derived address   --[249_964_000 collateral]---> lessee_collateral_account
   ```
   Where `249_964_000 = 249_998_800 − 34_800`.

   Both vaults close, their rent-exempt lamports go to the lessor. The
   `Lease` account closes via `close = lessor`.

**Final balances:**

- Lessor: 1 000 000 000 leased units (full return), 36 000 collateral units (total lease fees
  received in steps 3 + 5), plus the lamports from three account closes.
- Lessee: 100 000 000 leased units → 0 (all returned), collateral: started with
  1 000 000 000, spent 200 000 000 on initial deposit + 50 000 000 on
  top-up, got back 249 964 000, so holds 999 964 000 collateral units (net cost
  of 36 000 — exactly the total lease fees paid).

### 4.2 Liquidation path

Same setup. Steps 1 and 2 run identically.

3. Time jumps to `T + 300`. A keeper observes a new Pyth price update:
   the leased-in-collateral price has spiked to 4.0 (exponent 0, price
   = 4). At that price, the debt value is `100_000_000 × 4 =
   400_000_000` collateral units. The collateral
   pot is still ~`200_000_000` (minus some streamed lease fees).
   Maintenance ratio = `200/400 = 50%`, well below the required 120%.

   The keeper calls `pay_lease_fee` first is *not* required — `liquidate`
   settles accrued lease fees itself. It goes straight to `liquidate`.

4. **`liquidate`** at `T + 300`:
   - Lease fee due = 300 × 10 = 3 000 collateral units; collateral_amount = 200 000 000
     so `lease_fee_payable = 3 000`.
     ```
     collateral_vault program-derived address --[3_000 collateral]--> lessor_collateral_account
     ```
   - Remaining = 200 000 000 − 3 000 = 199 997 000 collateral units.
   - Bounty = 199 997 000 × 500 / 10 000 = 9 999 850 collateral units.
     ```
     collateral_vault program-derived address --[9_999_850 collateral]--> keeper_collateral_account
     ```
   - Lessor share = 199 997 000 − 9 999 850 = 189 997 150 collateral units.
     ```
     collateral_vault program-derived address --[189_997_150 collateral]--> lessor_collateral_account
     ```
   - Both vaults close; Lease closes. Status recorded as `Liquidated`.

**Final balances:**

- Lessor: 900 000 000 leased units (never got the 100 back — the
  lessee kept them), `3 000 + 189 997 150 = 190 000 150` collateral
  units, plus rent-exempt lamports from three closes.
- Lessee: *still* has 100 000 000 leased units. Spent 200 000 000 collateral units on
  deposit, got nothing back. Net: they walk away with the leased tokens
  but forfeited the entire collateral minus the keeper's cut.
- Keeper: 9 999 850 collateral units for their trouble.

(This is the key asymmetry: liquidation does *not* reclaim the leased
tokens. The collateral pays the lessor for the lost asset. The lessee
has effectively bought the leased tokens at the forfeit price.)

### 4.3 Falling-price path — borrower profits

Liquidation is a one-sided risk: it only ever fires when the leased
asset *appreciates* against the collateral. If the leased asset
depreciates, the collateral ratio rises and the borrower's position
gets safer. The streaming lending fee is the position's only ongoing
cost.

Same setup. Steps 1 and 2 run identically.

3. Time jumps to `T + 300`. The leased-in-collateral price has
   *fallen* to 0.5 (exponent 0, price = 0). To make the math
   non-trivial, take exponent = −1, price = 5: the debt value is
   `100_000_000 × 5 / 10 = 50_000_000` collateral units. The
   collateral pot is ~`200_000_000` (minus a tiny bit of streamed
   lease fees). Maintenance ratio = `200_000_000 / 50_000_000 =
   400%`, far above the required 120%.

   A keeper calling `liquidate` here would fail with
   `PositionHealthy` — the program refuses to seize a healthy
   position. The lessee is in the clear.

4. **`return_lease`** called at `T + 600` (10 minutes in). The
   lessee buys 100 leased tokens on the open market at the new price
   (about 50 collateral tokens total — far less than the 200
   collateral tokens they posted), then returns those tokens to
   close out the lease.

   Lease fees accrued: 600 × 10 = 6 000 collateral units.

   ```
   lessee_leased_account  --[100_000_000 leased]--> leased_vault program-derived address
   leased_vault program-derived address       --[100_000_000 leased]--> lessor_leased_account
   collateral_vault program-derived address   --[6_000 collateral]---------> lessor_collateral_account
   collateral_vault program-derived address   --[199_994_000 collateral]---> lessee_collateral_account
   ```

**Final balances:**

- Lessor: 1 000 000 000 leased units (full return), 6 000 collateral units in lease
  fees.
- Lessee: received 100 000 000 leased units, sold them at the
  original price, bought 100 leased tokens back at the lower price,
  returned them. Net cost is the lending fee (6 000 collateral units)
  plus whatever they paid on the open market for the replacement
  tokens; gain is the difference between the original sale price and
  the buy-back price. The standard short payoff.

The borrower can defend a borderline position with
`top_up_collateral` or close it early via `return_lease`. Only
adverse price moves trigger liquidation.

### 4.4 Default / expiry path — `close_expired` on an `Active` lease

Same setup. Steps 1 and 2 run as usual. The lessee takes the tokens,
posts collateral, then disappears.

3. `pay_lease_fee` is never called. Clock advances all the way past
   `end_timestamp = T + 86_400`.

4. **`close_expired`** called by the lessor at `T + 100_000`:
   - `status == Active` and `now >= end_timestamp` → the default branch runs.
   - `leased_vault` is empty (lessee kept the tokens). No transfer.
   - `collateral_vault` has 200 000 000 collateral units. All of it goes to the
     lessor:
     ```
     collateral_vault program-derived address --[200_000_000 collateral]--> lessor_collateral_account
     ```
   - Both vaults close; Lease closes.
   - `last_paid_timestamp = min(now, end_timestamp) = end_timestamp` (step added in
     Fix 5).

**Final balances:**

- Lessor: 900 000 000 leased units, 200 000 000 collateral units (the entire
  collateral pot as compensation), plus three account-close refunds.
- Lessee: 100 000 000 leased units, −200 000 000 collateral units. They paid the
  full collateral and kept the leased tokens.

### 4.5 Default / expiry path — `close_expired` on a `Listed` lease

This is the cheap cancel path. No lessee ever showed up.

1. `create_lease` as above.
2. `close_expired` called by the lessor immediately.
   - `status == Listed` → no expiry check.
   - `leased_vault` holds 100 000 000 leased units. Drain back:
     ```
     leased_vault program-derived address --[100_000_000 leased]--> lessor_leased_account
     ```
   - `collateral_vault` is empty. No transfer.
   - Both vaults close; Lease closes.

**Final balances:** lessor is back to 1 000 000 000 leased units; nothing
else moved.

---

## 5. Safety and edge cases

### 5.1 What the program refuses to do

All of the following come from [`errors.rs`](programs/asset-leasing/src/errors.rs)
and are enforced by either an Anchor constraint or a `require!` in the
handler:

| Error | When |
|---|---|
| `InvalidLeaseStatus` | Action tried against a lease in the wrong state (e.g. `take_lease` on a lease that is already `Active`) |
| `InvalidDuration` | `duration_seconds <= 0` on `create_lease` |
| `InvalidLeasedAmount` | `leased_amount == 0` on `create_lease` |
| `InvalidCollateralAmount` | `required_collateral_amount == 0` on `create_lease`; `amount == 0` on `top_up_collateral` |
| `InvalidLeaseFeePerSecond` | `lease_fee_per_second == 0` on `create_lease` |
| `InvalidMaintenanceMargin` | `maintenance_margin_basis_points == 0` or `> 50_000` on `create_lease` |
| `InvalidLiquidationBounty` | `liquidation_bounty_basis_points > 2_000` on `create_lease` |
| `LeaseExpired` | Reserved; not currently used (Lease fee accrual naturally caps at `end_timestamp`) |
| `LeaseNotExpired` | `close_expired` called on an `Active` lease before `end_timestamp` |
| `PositionHealthy` | `liquidate` called on a lease that passes the maintenance-margin check |
| `StalePrice` | Pyth price update older than 60 s, or has a future `publish_time`, or fails discriminator / length check |
| `NonPositivePrice` | Pyth price is `<= 0` |
| `MathOverflow` | Any of the `checked_*` arithmetic returned `None` |
| `Unauthorised` | Lease-modifying handler called by someone who is not the registered lessee (`top_up_collateral`, `return_lease`) |
| `LeasedMintEqualsCollateralMint` | `create_lease` called with the same mint for both sides |
| `PriceFeedMismatch` | `liquidate` called with a Pyth update whose `feed_id` does not match `lease.feed_id` |

### 5.2 Guarded design choices worth knowing

- **Leased tokens are locked up-front.** `create_lease` moves the tokens
  into the `leased_vault` immediately, so a lessee calling `take_lease`
  cannot fail because the lessor spent the funds elsewhere in the
  meantime.

- **Leased mint ≠ collateral mint.** If both sides used the same
  mint, the two vaults would hold the same asset and the
  "what-do-I-owe-vs-what-do-I-hold" accounting would collapse. The
  guard is cheap and the error message is explicit.

- **Feed pinning.** The Pyth `feed_id` is stored on the `Lease` at
  creation and enforced on every `liquidate`. A keeper cannot pass in a
  random unrelated price feed (like a volatile pair that happens to be
  dipping) to force a spurious liquidation.

- **Staleness window.** Pyth `publish_time` older than 60 seconds is
  rejected, and `publish_time > now` is rejected too (keepers must not
  front-run the validator clock).

- **Integer-only math.** Every percentage and price calculation folds
  into a `checked_mul` / `checked_div` of `u128` — no floats, no
  surprising NaN. `BPS_DENOMINATOR = 10 000` is the only
  "percentage denominator" anywhere; cross-check against `constants.rs`
  if you're porting the math.

- **Authority-is-self vaults.** `leased_vault.authority ==
  leased_vault.key()` (and likewise for `collateral_vault`). The
  program signs as the vault using its own seeds, which means the
  `Lease` account is not involved in signing any of the token moves.
  This keeps the signer-seed array small (one seed list, not two).

- **Max maintenance margin = 500%.** Without an upper bound a lessor
  could set a margin that is unreachable on day one and liquidate the
  lessee instantly. 50 000 basis points is generous — enough for truly
  speculative leases — while still blocking the pathological 10 000×
  trap.

- **Max liquidation bounty = 20%.** Higher than 20% and the keeper's
  cut would dwarf the lessor's recovery on default. The cap keeps
  liquidation economics roughly in line with lender-first semantics.

### 5.3 Things the program does *not* guard against

A production protocol would want more:

- **Price feed correctness.** The program verifies the owner
  (`PYTH_RECEIVER_PROGRAM_ID`), the discriminator, the layout and the
  feed id, but it cannot know whether the feed the lessor pinned
  quotes the right pair. Supplying the wrong feed at creation is the
  lessor's problem — it won't cause a liquidation to succeed against a
  truly healthy position (the feed id check would fail), but it will
  mean *no* liquidation can succeed, so a lessee could drain the
  collateral via lease fees and walk away. A production version would cross-
  check the price feed's `feed_id` against a protocol registry.

- **Lease-fee dust accumulation.** Lease fees are paid in whole base
  units per second of `lease_fee_per_second`. Choose a small
  `lease_fee_per_second` and short-lived leases can settle 0 lease
  fees if no-one calls `pay_lease_fee` for a very short period. Not a
  security issue — the accrual timestamp only moves forward when the lease
  fee is actually settled — but worth knowing.

- **Griefing on `init_if_needed`.** `take_lease`, `pay_lease_fee`,
  `liquidate`, `return_lease` and `close_expired` all do
  `init_if_needed` on one or more associated token accounts. If the caller does not fund
  the rent-exempt reserve for those accounts, the transaction fails.
  This is the intended behaviour (the caller pays for the state they
  require) but can surprise a lessee on a tight SOL budget.

- **No partial lease-fee refund on default.** When `close_expired` runs on
  an `Active` lease, the lessor takes the entire collateral regardless
  of how many lease fees had actually accrued by then. This is a deliberate
  simplification — the `last_paid_timestamp` bookkeeping in Fix 5 is in
  place precisely so a future version can split the pot correctly.

- **No pause / upgrade authority.** The program has no admin and no
  upgrade authority-bound feature flags. It runs or it doesn't.

---

## 6. Running the tests

All the tests are LiteSVM-based Rust integration tests under
[`programs/asset-leasing/tests/`](programs/asset-leasing/tests/). They
exercise every instruction handler through `include_bytes!("../../../target/deploy/asset_leasing.so")`,
so a fresh build must produce the `.so` first.

### Prerequisites

- Anchor 1.0.0 (`anchor --version`)
- Solana CLI (`solana -V`)
- Rust stable (the `rust-toolchain.toml` at the repo root pins the
  compiler)

### Commands

From this directory (`defi/asset-leasing/anchor/`):

```bash
# 1. Build the BPF .so — writes to target/deploy/asset_leasing.so
anchor build

# 2. Run the LiteSVM tests (just cargo under the hood; `anchor test`
#    also works because Anchor.toml scripts.test = "cargo test")
cargo test --manifest-path programs/asset-leasing/Cargo.toml

# Or, equivalently:
anchor test --skip-local-validator
```

Expected output:

```
running 11 tests
test close_expired_cancels_listed_lease ... ok
test close_expired_reclaims_collateral_after_end_timestamp ... ok
test create_lease_locks_tokens_and_lists ... ok
test create_lease_rejects_same_mint_for_leased_and_collateral ... ok
test liquidate_rejects_healthy_position ... ok
test liquidate_rejects_mismatched_price_feed ... ok
test liquidate_seizes_collateral_on_price_drop ... ok
test pay_lease_fee_streams_collateral_by_elapsed_time ... ok
test return_lease_refunds_unused_collateral ... ok
test take_lease_posts_collateral_and_delivers_tokens ... ok
test top_up_collateral_increases_vault_balance ... ok
```

### What each test exercises

| Test | Exercises |
|---|---|
| `create_lease_locks_tokens_and_lists` | Lessor funds vault, `Lease` created, collateral vault empty |
| `create_lease_rejects_same_mint_for_leased_and_collateral` | Guard against `leased_mint == collateral_mint` |
| `take_lease_posts_collateral_and_delivers_tokens` | Collateral deposit + leased-token payout in one instruction |
| `pay_lease_fee_streams_collateral_by_elapsed_time` | Lease fee math: `elapsed * lease_fee_per_second`, lease fee transferred to lessor |
| `top_up_collateral_increases_vault_balance` | Collateral balance after `top_up` equals deposit + top-up |
| `return_lease_refunds_unused_collateral` | Happy path round-trip — leased tokens returned, residual collateral refunded, accounts closed |
| `liquidate_seizes_collateral_on_price_drop` | Price-induced underwater position → lease fee + bounty + lessor share paid, accounts closed |
| `liquidate_rejects_healthy_position` | Program refuses to liquidate a position that passes the margin check |
| `liquidate_rejects_mismatched_price_feed` | Program refuses a `PriceUpdateV2` whose `feed_id` ≠ `lease.feed_id` |
| `close_expired_reclaims_collateral_after_end_timestamp` | Default path — lessor seizes the collateral |
| `close_expired_cancels_listed_lease` | Lessor-initiated cancel of an unrented lease |

### Note on CI

The repo's `.github/workflows/anchor.yml` runs `anchor build` before
`anchor test` for every changed anchor project. That's important for
this project: the Rust integration tests include the BPF artefact via
`include_bytes!`, so a stale or missing `.so` would break the tests.
CI is already covered.

---

## 7. Quasar port

A parallel implementation of the same program using
[Quasar](https://github.com/blueshift-gg/quasar) lives in
[`../quasar/`](../quasar/). Quasar is a lightweight alternative to
Anchor that compiles to bare Solana program binaries without pulling in
`anchor-lang` — useful when you care about compute-unit budget, binary
size, or simply want fewer layers between your code and the runtime.

The port implements the same seven instruction handlers, the same
`Lease` state account, the same program-derived address seed conventions, and produces the
same onchain behaviour for every happy-path and adversarial test in
this README.

### Building and testing

From [`../quasar/`](../quasar/):

```bash
# Build the .so using the quasar CLI.
quasar build

# Run the LiteSVM-style tests directly with cargo. The tests call the
# compiled program from `target/deploy/quasar_asset_leasing.so`.
cargo test
```

The Quasar example in this repo's CI workflow
(`.github/workflows/quasar.yml`) runs exactly those two commands.

### What differs from the Anchor version

- **No Anchor account-validation macros.** In Quasar, account structs
  use `#[derive(Accounts)]` with an almost-identical attribute
  vocabulary (`seeds`, `bump`, `has_one`, `constraint`,
  `init_if_needed`) but the checks are lowered to plain Rust, not
  inserted by a procedural macro that calls into a support crate.

- **Explicit instruction discriminators.** Each instruction handler
  carries `#[instruction(discriminator = N)]` with `N` an explicit
  integer — Quasar uses one-byte discriminators by default rather than
  Anchor's 8-byte sha256 prefix. The wire format for every call is
  `[discriminator: u8][borsh-serialised args]`.

- **Tests talk to `QuasarSvm` directly.** Instead of the Anchor
  `Instruction { ... }.data()` / `accounts::Foo { ... }.to_account_metas()`
  helpers, the Quasar tests build each `Instruction` by hand with
  `solana_instruction::AccountMeta` entries and a manually-assembled
  byte payload. Account state is pre-populated on the SVM with
  `QuasarSvm::new().with_program(...).with_token_program()` and
  helpers from `quasar_svm::token` that synthesise `Mint` and
  `TokenAccount` bytes without running the real token-program
  initialisation instruction handlers. This keeps the tests fast but
  means the setup code is more explicit.

- **No cross-program-invocation into an associated-token-account
  program for associated token account creation.** The Anchor version uses `init_if_needed`
  + `associated_token::...` to let callers pass in a lessor/lessee
  wallet and get the token account created on demand. The Quasar port
  accepts pre-created token accounts for the user side of every flow,
  since doing `init_if_needed` correctly for associated token accounts in Quasar requires
  wiring in the associated token account program manually and adds noise that distracts
  from the lease mechanics. Production code would want the associated token account
  convenience back.

- **Classic Token only, not Token-2022.** The Anchor version declares
  its token accounts as `InterfaceAccount<Token>` + `token_program:
  Interface<TokenInterface>`, which accepts mints owned by either the
  classic Token program or the Token-2022 program. The Quasar port
  uses `Account<Token>` + `Program<Token>`, matching the simpler
  pattern used by the other Quasar examples in this repo. Adding
  Token-2022 support is a type-parameter swap away.

- **State layout is the same, byte for byte.** The `Lease` discriminator
  and field order match the Anchor version, so an off-chain indexer
  that already decodes Anchor `Lease` accounts would also decode the
  Quasar ones after adjusting for the one-byte discriminator.

- **One lease per lessor at a time.** The Anchor version keys its
  `Lease` program-derived address on `[LEASE_SEED, lessor, lease_id]` so one lessor can
  run many leases in parallel. Quasar's `seeds = [...]` macro embeds
  raw references into generated code and does not (yet) have a
  borrow-safe way to splice instruction args like
  `lease_id.to_le_bytes()` into the seed list, so the Quasar port
  keys its program-derived address on `[LEASE_SEED, lessor]` alone — one active lease per
  lessor. The `lease_id` is still stored on the `Lease` account for
  book-keeping and is a caller-supplied u64 in `create_lease`; the
  off-chain client just has to ensure the previous lease from the same
  lessor is `Closed` or `Liquidated` (i.e. its program-derived address account is gone)
  before creating a new one. Swapping in a multi-lease seed is a
  mechanical change once Quasar grows support for dynamic-byte seeds.

The code layout mirrors this directory: `src/lib.rs` registers the
entrypoint and re-exports handlers, `src/state.rs` defines `Lease` and
`LeaseStatus`, and `src/instructions/*.rs` contains one file per
handler. Tests are in `src/tests.rs`.

---

## 8. Extending the program

Directions a real protocol would consider, grouped by effort:

### Easy

- **Add a `lease_view` read-only helper.** An off-chain indexer-style
  struct that returns `{ collateral_value, debt_value, ratio_basis_points,
  is_underwater }` given the same inputs `is_underwater` uses. Useful
  for UIs that want to show "you are 15% away from liquidation".

- **Cap lease fees at collateral.** Currently `pay_lease_fee` pays `min(lease_fee_due,
  collateral_amount)` and silently leaves a debt. Add an explicit
  `LeaseFeeDebtOutstanding` error so the caller is warned when the stream
  has stalled, rather than inferring it from a non-zero `lease_fee_due`
  after settlement.

### Moderate

- **Partial-refund default.** In `close_expired` on `Active`, instead
  of giving the lessor the entire collateral, split it:
  `lease_fee_due` to the lessor, the rest stays with the lessee up to some
  `default_haircut_basis_points`. `last_paid_timestamp` is already bumped by
  Fix 5, so the timestamp invariants are ready.

- **Multiple outstanding leases per `(lessor, lessee)` pair with the
  same mint pair.** Already supported via `lease_id`, but add an
  instruction-level index account that lists open lease ids for a
  given lessor so off-chain tools don't have to `getProgramAccounts`
  scan.

- **Quote asset ≠ collateral mint.** Rent and liquidation math assume
  debt is priced in *collateral units*. Generalise to a third "quote"
  mint by taking the price pair at creation and carrying a
  `quote_mint` pubkey on `Lease`. Requires updates to
  `is_underwater` and a second oracle feed.

### Harder

- **Keeper auction.** Replace the fixed `liquidation_bounty_basis_points` with a
  Dutch auction that grows the bounty linearly over some window after
  the position first becomes underwater. Keeps liquidators honest on
  tight feeds and gives lessees a chance to `top_up_collateral` before
  a keeper has an economic reason to move.

- **Flash liquidation.** Let the keeper settle the debt in the same
  transaction as the liquidation — borrow the leased amount from a
  separate liquidity pool, hand it to the lessor, take the full
  collateral, repay the pool, keep the spread. Requires integrating a
  second program.

- **Token-2022 support.** The program already uses the `TokenInterface`
  trait so it accepts mints owned by either the classic Token program
  or the Token-2022 program. A real extension would test against
  Token-2022 mint extensions (transfer-fee, interest-bearing) and
  document which are compatible with the lease-fee / collateral flows.

---

## Code layout

```
defi/asset-leasing/anchor/
├── Anchor.toml
├── Cargo.toml
├── README.md              (this file)
└── programs/asset-leasing/
    ├── Cargo.toml
    ├── src/
    │   ├── constants.rs    program-derived address seeds, basis points limits, Pyth age cap
    │   ├── errors.rs
    │   ├── lib.rs          #[program] entry points
    │   ├── instructions/
    │   │   ├── mod.rs
    │   │   ├── shared.rs           transfer / close helpers
    │   │   ├── create_lease.rs
    │   │   ├── take_lease.rs
    │   │   ├── pay_lease_fee.rs
    │   │   ├── top_up_collateral.rs
    │   │   ├── return_lease.rs
    │   │   ├── liquidate.rs
    │   │   └── close_expired.rs
    │   └── state/
    │       ├── mod.rs
    │       └── lease.rs
    └── tests/
        └── test_asset_leasing.rs   LiteSVM tests
```
