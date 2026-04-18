# Asset Leasing

Fixed-term leasing of SPL tokens on Solana, with SPL collateral, rent that
streams by the second, and Pyth-priced liquidation when the lessee stops
posting enough collateral.

This README is written as a **teaching document**. It assumes you have
written code before (you know what a function is, you can read basic Rust
or are happy to Google as you go) but it does **not** assume you know
anything about traditional finance. No previous Solana programs required
either — "this is my first program" is exactly the audience.

If you already know what collateral, maintenance margin, basis points,
liquidation and oracles mean, skip straight to the [Instructions
reference](#4-instructions-reference) or the [Lifecycle walk-through
](#3-the-full-lifecycle-walked-through-with-numbers).

---

## Table of contents

1. [What is asset leasing?](#1-what-is-asset-leasing)
2. [Key concepts, explained from scratch](#2-key-concepts-explained-from-scratch)
3. [The full lifecycle, walked through with numbers](#3-the-full-lifecycle-walked-through-with-numbers)
4. [Instructions reference](#4-instructions-reference)
5. [Accounts and PDAs](#5-accounts-and-pdas)
6. [Pyth integration, in plain English](#6-pyth-integration-in-plain-english)
7. [Safety and edge cases worth knowing](#7-safety-and-edge-cases-worth-knowing)
8. [How to build and test](#8-how-to-build-and-test)
9. [Extending this example](#9-extending-this-example)
10. [Further reading](#10-further-reading)

---

## 1. What is asset leasing?

The short version: **renting a token instead of buying it, with a refundable
security deposit that keeps the renter honest.**

### A real-world analogy

Imagine you want to drive a car for a week but you do not want to buy one.
You walk into a rental place. They hand you the keys, but only after you
leave a **security deposit** on your credit card and agree to a **daily
rate**. Every day you keep the car, they bill you for another day. If the
car's value suddenly crashes (they discover it has a bent chassis), they
might ask for a **bigger deposit** so they are still protected. If you
never return the car, they **keep your deposit** to cover the loss. If
you return it on time and pay the rent you owe, they hand your deposit
back, minus the days you used.

Asset leasing on Solana is the same shape:

- The **lessor** (the car rental company) owns some SPL tokens and is
  happy to lend them out for a fee.
- The **lessee** (the renter) wants those tokens for a while but does not
  want to buy them outright.
- The lessee posts **collateral** (security deposit) in a different SPL
  token, and in exchange gets the leased tokens.
- **Rent** ticks away per second, paid out of that collateral into the
  lessor's wallet.
- If the market value of the collateral drops versus the leased tokens,
  the lessee must **top up** or risk being **liquidated** — a third
  party ("keeper") takes the collateral, the lessor is made whole, and
  the keeper earns a small fee for the trouble.
- If the lessee disappears and never returns the tokens, the lessor
  **keeps the whole collateral** to compensate for the loss.

### Why would anyone want to do this?

A concrete example — **governance leasing**.

> Alice owns 10,000 tokens of a DAO's governance token. She does **not
> want to sell** them — they grow in value, she likes the project, and
> she wants to keep her long-term position. But right now she is not
> voting on anything and the tokens sit idle.
>
> Bob, meanwhile, wants to vote on an upcoming proposal. He does not
> want to spend the cash to buy 10,000 tokens just to vote once. What
> he does want is **temporary voting power for a week**.
>
> They lease. Alice posts her 10,000 tokens for rent. Bob takes the
> lease, hands over USDC as collateral, gets the 10,000 governance
> tokens, votes, and returns them a week later. Alice earns a week of
> rent in USDC for doing nothing. Bob got voting power without burning
> capital on tokens he does not want to hold long-term. Everyone is
> happier than they would have been otherwise.

The same pattern works for:

- **NFT utility rental** — rent a game item, an access pass, or a
  profile-picture NFT for a weekend.
- **Collateral rental** — borrow an asset to use as collateral elsewhere
  (rehypothecation, in traditional-finance speak), return it later.
- **Voting / boost rental** — "ve-token" systems where holding the
  token gives extra yield; a DAO that wants a temporary boost can lease
  instead of buying.

The onchain bit matters because there is **no trusted middleman**. The
program enforces the rules: the collateral is locked, the rent is paid
automatically, and if either party misbehaves the other has an
onchain remedy. You do not have to trust Alice not to run off with
Bob's deposit, and Alice does not have to trust Bob to return the
tokens — the program enforces both sides.

---

## 2. Key concepts, explained from scratch

Before reading the code, it helps to know what these words mean. None of
them are complicated — they are just jargon the finance world uses, and
the code re-uses the same vocabulary.

### 2.1 SPL tokens

On Ethereum you have ERC-20 tokens. **SPL tokens are the Solana
equivalent** — a common standard so any wallet, any program, and any UI
can speak to any token the same way. Both the leased asset and the
collateral in this example are SPL tokens. USDC on Solana is an SPL
token. Wrapped SOL is an SPL token. A DAO's governance token is
(probably) an SPL token. You can mint your own in a few lines. The
program does not care which mint is used — only that both sides agreed
on them at listing time.

### 2.2 Collateral

Collateral is a **security deposit**. It is something valuable the
borrower gives up temporarily to reassure the lender.

Why demand it? Because once the lessee has your tokens, nothing except
collateral stops them from walking away. "Skin in the game" is the
phrase — the lessee has something to lose, so cooperating (returning
the tokens on time, paying rent) is more profitable than defecting
(keeping the tokens and losing the deposit).

In this program the collateral lives in a program-owned `collateral_vault`.
The lessee cannot touch it. The program will only release it under the
rules defined in the code (return, top-up withdrawal, liquidation, or
expiry).

### 2.3 Maintenance margin

This is where the finance vocabulary bites, but the idea is simple.

**The collateral must stay worth more than the thing being borrowed.**
How much more? That is the maintenance margin, expressed as a
percentage (well, a basis-point number — see §2.6).

In this program, the margin is stored as `maintenance_margin_bps`. If
you set it to `12_000`, you are saying:

> collateral value must be ≥ 120% of the leased-asset value at all
> times. If it falls below 120%, the position is **underwater** and can
> be liquidated.

Worked example (same mint for simplicity):

- Alice lists 100 GOV tokens.
- The maintenance margin is 120% (`12_000` bps).
- Right now 1 GOV = 1 USDC. So the debt is 100 USDC.
- Required collateral to stay healthy: 100 × 120% = **120 USDC**.
- Bob posts 200 USDC to be safe.

A week later GOV pumps to 1.80 USDC:

- Debt value is now 100 × 1.80 = 180 USDC.
- Required collateral: 180 × 120% = **216 USDC**.
- Bob only has 200 locked up. He is now **underwater**. Unless he tops
  up, a keeper can liquidate him.

Why the margin exists: price quotes are stale by the time you see them,
and a 0.01% cushion is not enough to cover even a small move. A 120%
or 150% margin gives the lessor a buffer against the inevitable price
swings between liquidation checks.

### 2.4 Liquidation

Liquidation is **the eject button**. When a position breaches the
maintenance margin, the protocol seizes the collateral, pays the
lessor what they are owed, and closes the position.

Why does the lessee not just volunteer to close? Because by the time
they are underwater, defaulting might be cheaper for them than
returning the tokens. Liquidation makes sure the lessor still gets
paid even when the lessee disappears.

**Here is a crucial Solana detail**: Solana programs cannot run
themselves. They have no background thread, no cron, no "trigger when
price changes". Every bit of code runs only because someone (a wallet,
a bot) sent a transaction that called an instruction.

So the program cannot "automatically" liquidate anyone. It has to wait
for someone to send a `liquidate` transaction, providing fresh price
data, and *then* it decides whether the liquidation is valid. This
is why we need keepers.

### 2.5 Keepers and the keeper bounty

A **keeper** is a bot (or a person) that watches the chain, spots
leases that have gone underwater, and sends `liquidate` transactions
to clean them up. Keepers are not special — they are not on some
whitelist, they have no privileged access. They are just accounts
with a script.

Why would anyone run a keeper? Because the program pays them. On a
successful liquidation, a configurable **bounty** (a small share of
the seized collateral, capped at 20% by `MAX_LIQUIDATION_BOUNTY_BPS`)
goes to the keeper. Everyone else's share is smaller as a result, so
the lessor does not love paying it, but it is strictly better than
having no keeper at all and letting the position rot.

This is a common DeFi pattern: **economic incentives replace trusted
operators**. Instead of hiring someone to watch positions, you write
a rule that pays whoever notices first, and the market takes care of
the rest. Keepers compete on speed; the lessor pays a tiny toll to
keep the system honest.

### 2.6 Basis points (bps)

The finance world avoids "percent" when precision matters. Instead it
uses **basis points**: 1 bp = 0.01%, 100 bps = 1%, **10,000 bps = 100%**.

Why? Mostly two reasons:

- **Clarity.** "Raise rates by 25 bps" is unambiguous. "Raise rates by
  0.25 percent" could mean 0.25 percentage points (reasonable) or
  0.25% of the current rate (much smaller). Bps never have that
  problem.
- **Integer arithmetic.** On Solana, floating-point is slow and has
  rounding surprises. Integer math is cheap and exact. If you express
  ratios as bps you can do `amount * bps / 10_000` with plain `u64`s
  and never touch a float.

This program uses bps everywhere a ratio appears:

- `maintenance_margin_bps` — e.g. `12_000` for 120%.
- `liquidation_bounty_bps` — e.g. `500` for 5%.
- The constant `BPS_DENOMINATOR` (= `10_000`) is the divisor.

### 2.7 Oracles — why programs need them

A Solana program is a pure function: given the accounts you hand it,
it computes a result. It **cannot call out** to an external API, it
**cannot read** a price from CoinGecko, and it does not have a
"world model" with current prices baked in.

So how does the program know whether a position is underwater? It
needs someone to **push the price onchain** for it to read. That
someone is an **oracle**.

**Pyth** is one of the main oracle networks on Solana. A set of
trusted publishers submit their prices every few seconds to the Pyth
program on Solana. Pyth then aggregates them into a single "official"
price, and stores the result in a special account you can read like
any other — the `PriceUpdateV2` account.

When the keeper calls `liquidate`, they pass in a freshly-updated
`PriceUpdateV2` account. The program:

1. Checks the account is actually owned by Pyth's receiver program
   (not a malicious account someone minted).
2. Reads the price out of it.
3. Checks that the price is not stale (more on this in §2.8).
4. Uses the price to compute `collateral_value / debt_value`.
5. Compares that to the maintenance margin.

Without an oracle, the program has no idea what "worth more" means —
it only sees token amounts, not token values. The price feed is the
bridge from "100 GOV, 200 USDC" to "the collateral is worth 2×
the debt".

> **Note.** The program trusts the keeper to supply a feed that
> actually quotes the leased asset in collateral units. Nothing in the
> code pins a specific feed to a specific lease — that would be a
> sensible upgrade (see §9).

### 2.8 Per-second rent

You could imagine a lease where the lessee pays a flat lump sum up
front, regardless of whether they use it for a minute or a month.
That is simple but wasteful: a lessee who returns early gets nothing
back, a lessee who runs late pays the same as one who returns on time.

This program instead does **streaming rent**: rent accrues by the
second. Every time anyone calls `pay_rent`, the program computes
`rent_per_second × seconds_elapsed_since_last_payment`, pulls that
out of the collateral vault, and sends it to the lessor.

From the code:

```rust
pub fn compute_rent_due(lease: &Lease, now: i64) -> Result<u64> {
    let cutoff = now.min(lease.end_ts);
    if cutoff <= lease.last_rent_paid_ts {
        return Ok(0);
    }
    let elapsed = (cutoff - lease.last_rent_paid_ts) as u64;
    elapsed
        .checked_mul(lease.rent_per_second)
        .ok_or(AssetLeasingError::MathOverflow.into())
}
```

Note two details:

- `cutoff = now.min(end_ts)` — rent stops accruing after the lease
  ends. If the lessee is 3 days late, they only owe rent up to
  `end_ts`, not for the late days. (The lessor gets the whole
  collateral by default, so "late fees" are not needed here.)
- The caller sends `pay_rent` whenever they like, but `now` on Solana
  is taken from the **clock sysvar**, not from the caller. So the
  lessee cannot cheat by sending early transactions with a fake
  timestamp.

### 2.9 PDAs — how a program owns things without a private key

A Program Derived Address is a **public key with no private key**.
It is deterministically computed from a program id plus some "seeds"
(arbitrary byte strings you choose). Because there is no private key,
no wallet can sign for it — except the program that owns it, which
can sign by replaying the seeds.

In this codebase you will see seeds like:

```rust
pub const LEASE_SEED: &[u8] = b"lease";
pub const LEASED_VAULT_SEED: &[u8] = b"leased_vault";
pub const COLLATERAL_VAULT_SEED: &[u8] = b"collateral_vault";
```

And in the account constraints, something like:

```rust
seeds = [LEASE_SEED, lessor.key().as_ref(), &lease_id.to_le_bytes()],
```

That address is **deterministic**: given the program id, the lessor's
pubkey, and `lease_id`, everyone in the world can compute the same
PDA. Convenient for UIs ("the lease for (lessor=X, id=7) lives
*here*") and impossible to spoof (nobody can beat the program to the
address).

### 2.10 Vault accounts

The `leased_vault` and `collateral_vault` are **SPL token accounts
whose authority is a PDA**. Ordinary tokens sit in wallets. These
tokens sit in accounts the program controls: only the program can
sign for moves out of them, and only under the rules the program
encodes.

From `create_lease.rs`:

```rust
#[account(
    init,
    payer = lessor,
    seeds = [LEASED_VAULT_SEED, lease.key().as_ref()],
    bump,
    token::mint = leased_mint,
    token::authority = leased_vault,   // <- vault is its own authority
    token::token_program = token_program,
)]
pub leased_vault: Box<InterfaceAccount<'info, TokenAccount>>,
```

Note the authority is the vault itself — a self-authorising PDA. The
handler moves tokens out by re-deriving the seeds and using them to
"sign" the CPI (cross-program invocation) to the token program:

```rust
let leased_vault_seeds: &[&[u8]] = &[
    LEASED_VAULT_SEED,
    lease_key.as_ref(),
    core::slice::from_ref(&leased_vault_bump),
];
let signer_seeds = [leased_vault_seeds];
transfer_tokens_from_vault(
    &context.accounts.leased_vault,
    &context.accounts.lessee_leased_account,
    leased_amount,
    &context.accounts.leased_mint,
    &context.accounts.leased_vault.to_account_info(),
    &context.accounts.token_program,
    &signer_seeds,
)?;
```

The "signature" here is not a crypto signature in the traditional
sense — it is the runtime saying "yes, this program is allowed to
move these tokens because it re-derived the seeds that produced the
authority, so it must be the program that set them up".

---

## 3. The full lifecycle, walked through with numbers

Let's follow a single lease through every path the program supports.
To keep the numbers friendly, we will use two fake SPL tokens:

- **GOV** — a governance token, 6 decimals. Current price: 1 USDC each.
- **USDC** — everyone's favourite stablecoin, 6 decimals. Worth 1 USDC
  (obviously).

Meet the cast:

- **Alice** — lessor. Owns 1,000 GOV and does not want to sell.
- **Bob** — lessee. Wants GOV temporarily, has some USDC.
- **Kim** — keeper. Runs a bot. Has no direct interest in either
  side, but will happily pocket a bounty for noticing if Bob goes
  underwater.

All amounts below are in whole-token units for readability; in the
actual test the same logic runs in 6-decimal base units.

### 3.1 Listing — `create_lease`

Alice calls `create_lease` with:

| Parameter | Value | Meaning |
| --- | --- | --- |
| `lease_id` | `1` | Unique per-lessor id so she can run multiple leases |
| `leased_amount` | `1_000` GOV | Amount she wants to lease out |
| `required_collateral_amount` | `1_200` USDC | Deposit Bob must post |
| `rent_per_second` | `1` USDC | Rent rate — ridiculously high for clarity |
| `duration_seconds` | `604_800` | One week |
| `maintenance_margin_bps` | `12_000` | 120% |
| `liquidation_bounty_bps` | `500` | 5% of the remaining collateral |

What happens:

1. The program creates three new accounts: the `Lease` PDA, the
   `leased_vault` (for GOV), and the `collateral_vault` (for USDC).
2. It transfers 1,000 GOV from Alice's wallet into the `leased_vault`.
   **This happens at listing, not at take-up**, so a lessee can never
   accept a lease where the lessor does not actually have the goods.
3. It records all the terms on the `Lease` account, with `status =
   Listed`, `lessee = Pubkey::default()`, `collateral_amount = 0`.

At this point Alice's wallet has 1,000 fewer GOV. They are escrowed.

### 3.2 Taking — `take_lease`

Bob sees Alice's listing (via a UI that queries `Lease` accounts).
He decides to take it. He calls `take_lease`. The program:

1. Verifies the lease is `Listed` and that the mints match.
2. Transfers 1,200 USDC from Bob's wallet into the `collateral_vault`.
3. Transfers 1,000 GOV out of the `leased_vault` into Bob's GOV ATA
   (created on the fly if he did not have one).
4. Updates the `Lease`: `lessee = Bob`, `collateral_amount = 1_200`,
   `start_ts = now`, `end_ts = now + 604_800`, `last_rent_paid_ts =
   now`, `status = Active`.

Now:

- The `leased_vault` is empty (Bob has the GOV).
- The `collateral_vault` holds 1,200 USDC.
- Alice has neither the GOV nor the USDC; she has an onchain claim
  to rent and, eventually, the GOV back.

### 3.3 Happy path — return on time

Two days in (172,800 seconds) Bob has finished his governance voting
and wants to return the tokens. A few things might have happened in
between:

- Someone (Bob, or a public-spirited keeper, or Alice herself) may
  have called `pay_rent` from time to time, streaming rent from the
  vault to Alice. It is not required — the tally is cumulative.
- The price of GOV may have moved either way. As long as the position
  stayed above the maintenance margin, nothing special happened.

Bob calls `return_lease`. The program:

1. Transfers 1,000 GOV from Bob back into the `leased_vault`, then
   straight out to Alice.
2. Computes accrued rent since `last_rent_paid_ts`. In this example:
   172,800 s × 1 USDC/s = 172,800 USDC of rent — clearly more than
   the 1,200 USDC collateral, so the program caps it at the vault
   balance. (Obviously you would choose a sensible rent rate in
   practice; these numbers are just to illustrate.)
3. Pays the rent to Alice, refunds **the rest of the collateral** to
   Bob.
4. Closes both vaults and the `Lease` account, sending the
   rent-exempt lamports back to Alice.

Key point: **Bob never pays rent for time he did not use.** The
`cutoff = now.min(end_ts)` in `compute_rent_due` and the "return
early" code path together guarantee it.

(In realistic numbers — say `rent_per_second = 10` base-units of
USDC — 172,800 s of rent is 1.728 USDC, Alice gets that, Bob gets
the other ~1,198.27 USDC back. Much happier arithmetic.)

### 3.4 Margin-call path — Bob tops up

Halfway through the week, GOV moons from 1.00 to 1.50 USDC. Now:

- Debt value: 1,000 GOV × 1.50 = 1,500 USDC.
- Required cushion at 120%: 1,800 USDC.
- Bob only has 1,200 USDC locked.

If any keeper calls `liquidate` with a fresh price, the program will
agree the position is underwater and seize the collateral. Bob does
not want that — he wants to finish his week. So he calls
`top_up_collateral` with `amount = 700` USDC. That tops the vault up
to 1,900 USDC, back above the 1,800 requirement. Any liquidation
attempt will now be rejected with `PositionHealthy`.

The code here is small and boring — exactly the sign of a good
function:

```rust
pub fn handle_top_up_collateral(context: Context<TopUpCollateral>, amount: u64) -> Result<()> {
    require!(amount > 0, AssetLeasingError::InvalidCollateralAmount);
    transfer_tokens_from_user(...)?;
    context.accounts.lease.collateral_amount = context
        .accounts
        .lease
        .collateral_amount
        .checked_add(amount)
        .ok_or(AssetLeasingError::MathOverflow)?;
    Ok(())
}
```

Note only the `lessee` can top up their own lease (`constraint =
lease.lessee == lessee.key()`).

### 3.5 Liquidation path — Bob does nothing

Same setup, but Bob is either asleep, out of USDC, or hoping the
price will come back. He does not top up. A keeper (Kim) is watching
and notices the lease is now underwater. She submits `liquidate` with
a recent `PriceUpdateV2` account quoting GOV at 1.50 USDC.

The program:

1. Verifies the `PriceUpdateV2` account is owned by the Pyth receiver
   program (rejects anything else).
2. Decodes the price and publish time. Rejects if the price is
   stale (> 60 s old) or in the future or non-positive.
3. Computes whether the position is underwater:
   `collateral_value × 10_000 < debt_value × margin_bps`.
   With 1,200 USDC vs. 1,500×1.20 = 1,800 USDC, yes — underwater.
4. Pays accrued rent to Alice first (so she gets paid for the time
   Bob did use).
5. Takes the **remaining** collateral and slices off the bounty:
   5% × remaining → Kim. The rest → Alice.
6. Closes both vaults and the lease account. `status = Liquidated`,
   `collateral_amount = 0`.

Notice: the leased vault is empty in this path. Bob kept the GOV.
Alice's compensation is purely in collateral. That is by design —
the collateral exists specifically to cover this case. If the
margin was set high enough to begin with, Alice is whole.

### 3.6 Default path — the week runs out, Bob ghosts

Bob never returns the tokens and never gets liquidated (maybe the
price held steady, so no keeper had cause to liquidate him). A week
later `end_ts` passes. The lease is still `Active`, but by its own
terms it has expired.

Alice calls `close_expired`. The program:

1. Checks `lease.status` is `Active` (or `Listed` — same instruction
   handles both).
2. If `Active`, requires `now >= end_ts`.
3. Drains whatever is in the leased vault back to Alice. (In this
   default case: zero — Bob has the GOV.)
4. Drains the collateral vault back to Alice. (In this case: the
   full 1,200 USDC, because no rent was settled.)
5. Closes both vaults and the lease.

Alice is out 1,000 GOV but has gained 1,200 USDC. As long as the
lease was priced correctly when created (collateral > leased value),
that is a fair outcome for her.

> **Heads up — a subtlety:** `close_expired` does not settle rent.
> In the default path all the collateral goes to Alice anyway, so
> the distinction does not matter. But if you ever extend the
> instruction (e.g. to refund *partial* collateral to the lessee),
> you will need to decide whether to call `compute_rent_due` first.

### 3.7 Cancelled listing — Alice changes her mind

Alice lists the lease and then decides not to rent it out. She can
reclaim her GOV any time before anyone takes it:

- She calls `close_expired` on the `Listed` lease.
- The `now >= end_ts` check does **not** apply to `Listed` leases
  (look at the `if status == Active` guard in the handler — listed
  leases skip it).
- The leased vault holds her 1,000 GOV; they go straight back to
  her. The collateral vault was never funded, so that part is a
  no-op. Both vaults and the lease close.

---

## 4. Instructions reference

There are seven instructions. They map one-to-one onto a file under
`programs/asset-leasing/src/instructions/`.

| Instruction | Who calls | What it does | Why it exists |
| --- | --- | --- | --- |
| `create_lease` | Lessor | Locks the leased tokens in a program vault and records the terms. Lease starts `Listed`. | Listing up front (rather than on take-up) means a lessee cannot accept a lease the lessor can no longer honour. The tokens are real, escrowed, and nobody but the program can move them. |
| `take_lease` | Lessee | Deposits `required_collateral_amount`, receives the leased tokens. Sets `start_ts`, `end_ts`, `last_rent_paid_ts`. Status → `Active`. | Two-step listing-then-take lets the lessor advertise terms without pre-committing to a specific lessee; anyone can take it first. |
| `pay_rent` | **Anyone** | Computes rent since `last_rent_paid_ts`, transfers it from the collateral vault to the lessor, updates `last_rent_paid_ts`. | Keeping the rent-accrual separate from return/liquidation lets the lessor (or anyone else) settle rent whenever — handy for long leases. The caller pays transaction fees, but no state is otherwise affected if there is no rent to move. |
| `top_up_collateral` | Lessee | Adds more collateral to the vault. | Market prices move. Without a top-up, a short-lived dip during a volatile hour would liquidate every lessee. Top-ups give the lessee a chance to defend their position. |
| `return_lease` | Lessee | Returns the full `leased_amount`, pays final rent, refunds remaining collateral, closes the lease. | The cooperative way to end a lease early or on time. Runs all the settlements in one transaction so there is no window where, say, the tokens have been returned but the collateral is still locked. |
| `liquidate` | Keeper | Verifies the supplied Pyth price, checks the position is underwater, pays accrued rent + bounty + lessor share, closes the lease. Status → `Liquidated`. | The non-cooperative way to end the lease when the collateral no longer covers the debt. Lessor is compensated, keeper is rewarded for doing the work. |
| `close_expired` | Lessor | Two modes: (1) cancel a `Listed` lease to reclaim the leased tokens, (2) sweep collateral + any remaining tokens after a defaulted `Active` lease's `end_ts`. | Lessors need an unclog path — "nobody is taking this, give me my tokens back" — and a default recovery path — "they never returned the tokens, pay me the collateral". Same instruction covers both; the branch is on `lease.status`. |

Each instruction has its own `Accounts` struct listing every account it
touches, with Anchor constraints (`has_one = lessor`, `constraint =
lease.status == LeaseStatus::Active`, etc.) that run **before** the
handler body. If any constraint fails, the transaction aborts and no
state changes. This is the usual Anchor pattern: put the validation in
the struct, keep the handler body about the business logic.

### Calling conventions

- `create_lease`, `take_lease`, `top_up_collateral`, `pay_rent`,
  `return_lease` and `close_expired` take the `lease_id` implicit in
  the seed-derived `Lease` PDA — the client derives the PDA and passes
  it as an account. Only `create_lease` needs `lease_id` as an explicit
  argument because the `Lease` PDA does not exist yet to derive it
  from.
- All account struct definitions live with their handlers; all of them
  are re-exported from `instructions/mod.rs` so `lib.rs` can use them
  by name.

---

## 5. Accounts and PDAs

Three PDAs hold the entire lifecycle of one lease.

### 5.1 `Lease` — the state account

Seeded by `(b"lease", lessor, lease_id)`. One lessor can run as many
leases in parallel as they like by using different `lease_id` values.

Fields (see `src/state/lease.rs` for the authoritative definition):

| Field | Type | Meaning |
| --- | --- | --- |
| `lease_id` | `u64` | Caller-chosen id, part of the PDA seed. |
| `lessor` | `Pubkey` | Who owns this lease. Receives rent; recovers assets. |
| `lessee` | `Pubkey` | Set by `take_lease`. `Pubkey::default()` while `Listed`. |
| `leased_mint` | `Pubkey` | Mint of the tokens being rented out. |
| `leased_amount` | `u64` | Fixed at creation. Always the same amount is returned. |
| `collateral_mint` | `Pubkey` | Mint of the collateral. |
| `collateral_amount` | `u64` | Live balance of the collateral vault as the program sees it. Increases on top-up, decreases as rent is paid. |
| `required_collateral_amount` | `u64` | Amount the lessee had to post up front. Not the same as `collateral_amount` — the vault's balance changes over time. |
| `rent_per_second` | `u64` | Streaming rate in collateral base-units per second. |
| `duration_seconds` | `i64` | Length of the lease, set at creation. |
| `start_ts`, `end_ts` | `i64` | Filled by `take_lease`. `0` while `Listed`. |
| `last_rent_paid_ts` | `i64` | Point up to which rent has been settled. |
| `maintenance_margin_bps` | `u16` | Health threshold. Capped at `MAX_MAINTENANCE_MARGIN_BPS` = 50_000 (500%). |
| `liquidation_bounty_bps` | `u16` | Keeper bounty. Capped at `MAX_LIQUIDATION_BOUNTY_BPS` = 2_000 (20%). |
| `status` | `LeaseStatus` | `Listed` → `Active` → `Closed`/`Liquidated`. |
| `bump`, `leased_vault_bump`, `collateral_vault_bump` | `u8` | Cached bump seeds so CPIs can re-sign without re-deriving. |

The lifecycle transitions (copied from the doc comment on
`LeaseStatus`):

```
Listed  --take_lease-->     Active
Active  --return_lease-->   Closed
Active  --liquidate-->      Liquidated
Listed  --close_expired-->  Closed   (lessor cancels)
Active  --close_expired-->  Closed   (defaulted lessee, after end_ts)
```

Why is it a PDA? So that:

- There is a **canonical address** anyone can derive. A UI does not
  need a database to find Alice's lease #1 — it computes
  `find_program_address([b"lease", alice, 1u64.to_le_bytes()], program_id)`
  and reads.
- The program can **act as authority** for the account without a
  private key. No mnemonic to hide, no key to lose, and no way for
  an attacker to forge a signature.
- Collisions are impossible: for the same `(lessor, lease_id)` pair
  there is exactly one PDA. `create_lease` uses `init`, which fails
  if the account already exists.

### 5.2 `leased_vault` — escrow for the leased tokens

An SPL token account. Seeded by `(b"leased_vault", lease)`. Authority
is itself (`token::authority = leased_vault`).

- Holds the `leased_amount` while the lease is `Listed`.
- Drained on `take_lease` (tokens go to lessee) and on `return_lease`
  (tokens go back to lessor via this vault).
- Empty during the `Active` phase (the lessee is holding the
  tokens).
- Closed when the lease settles, freeing its rent-exempt lamports
  back to the lessor.

### 5.3 `collateral_vault` — escrow for the collateral

An SPL token account. Seeded by `(b"collateral_vault", lease)`.
Authority is itself.

- Funded on `take_lease` with `required_collateral_amount`.
- Grows on `top_up_collateral`.
- Shrinks on every `pay_rent` (rent flows lessor-ward).
- Split between lessor and keeper on `liquidate`.
- Fully drained on `return_lease` / `close_expired`.
- Closed at settlement, lamports to the lessor.

### 5.4 Associated Token Accounts

The program touches several **associated token accounts** (ATAs) —
token accounts deterministically derived from `(wallet, mint)`. You
will see `associated_token::mint = X, associated_token::authority =
Y` in account structs. These are not PDAs *of this program*; they are
PDAs of the Associated Token Account program, a standard way for any
wallet to have "the account" for any given mint.

Many of the ATAs are marked `init_if_needed` — meaning "create the
account if it does not exist yet, otherwise use the existing one".
This is a small quality-of-life touch: the UI never has to
pre-create token accounts for users, the program does it when it
first needs one. The caller pays the rent for the new account.

---

## 6. Pyth integration, in plain English

### 6.1 What Pyth is

**Pyth** is an oracle network. Professional trading firms, exchanges
and market makers run Pyth publisher nodes that submit prices several
times a second. On Solana, the aggregated result is written into
`PriceUpdateV2` accounts you can read from your program like any
other account.

The key idea: Pyth does not guess prices. Real market participants
report prices from their own systems, Pyth aggregates, signs, and
posts. Your program treats the result as **ground truth for this
block**.

### 6.2 The `PriceUpdateV2` account

A `PriceUpdateV2` is a regular Solana account, owned by the Pyth
Solana Receiver program
(`rec5EKMGg6MxZYaMdyBfgwp4d5rB9T1VQH5pJv5LtFJ`). Its data layout is
(simplified):

```
| 8  bytes | Anchor discriminator for "PriceUpdateV2"
| 32 bytes | write_authority
| 1  byte  | verification_level
| 32 bytes | feed_id
| 8  bytes | price                (i64)
| 8  bytes | conf                 (u64)  — not used here
| 4  bytes | exponent             (i32)
| 8  bytes | publish_time         (i64)
... more fields we don't read
```

A Pyth price is stored as an integer `price` plus an integer
`exponent`, such that the real price is `price × 10^exponent`. A
`price = 15_000, exponent = -4` means `1.5000`. This keeps the value
exact — no floats anywhere.

### 6.3 Why our program decodes by hand

Normally you would import `pyth-solana-receiver-sdk` to parse the
account. We don't, and the comment at the top of `liquidate.rs`
tells you why:

> We do not pull in `pyth-solana-receiver-sdk` because that crate
> currently has a transitive `borsh` conflict with `anchor-lang`
> 1.0.0 (see `program-examples/.github/.ghaignore` — `oracles/pyth/anchor`
> is flagged for the same reason).

Anchor 1.0 upgraded Borsh in a way that Pyth's SDK has not caught up
to yet. Rather than fight dependency resolution, we hard-code the
account layout and read the bytes ourselves:

```rust
pub const PRICE_UPDATE_V2_DISCRIMINATOR: [u8; 8] =
    [34, 241, 35, 99, 157, 126, 244, 205];

pub fn decode_price_update(data: &[u8]) -> Result<DecodedPriceUpdate> {
    const PRICE_OFFSET: usize = 73;
    const EXPONENT_OFFSET: usize = PRICE_OFFSET + 8 + 8;
    const PUBLISH_TIME_OFFSET: usize = EXPONENT_OFFSET + 4;
    const MIN_LEN: usize = PUBLISH_TIME_OFFSET + 8;

    require!(data.len() >= MIN_LEN, AssetLeasingError::StalePrice);
    require!(
        data[..8] == PRICE_UPDATE_V2_DISCRIMINATOR,
        AssetLeasingError::StalePrice
    );
    // ... i64::from_le_bytes / i32::from_le_bytes out of fixed offsets
}
```

Two safety properties make this safe:

1. The `Accounts` struct declares `#[account(owner =
   PYTH_RECEIVER_PROGRAM_ID)]`. Solana's runtime refuses the
   transaction if anyone passes an account not owned by Pyth's
   receiver program. So the bytes we read are produced by Pyth, not
   by an attacker.
2. The first 8 bytes are checked against the `PriceUpdateV2`
   discriminator. That rejects any Pyth-owned account of a
   *different* type (in case future Pyth versions add sibling
   account types).

If the SDK dependency issue ever gets resolved upstream, future-you
can swap the manual decode for a type-checked `PriceUpdateV2::try_from`
call. The handler's shape won't change.

### 6.4 Staleness

Prices are only useful if they are recent. An old price can make a
healthy position look unhealthy, or (much worse) let a keeper
liquidate based on a stale snapshot.

From `constants.rs`:

```rust
pub const PYTH_MAX_AGE_SECONDS: u64 = 60;
```

and from `is_underwater` in `liquidate.rs`:

```rust
require!(price.publish_time <= now, AssetLeasingError::StalePrice);
let age = (now - price.publish_time) as u64;
require!(age <= PYTH_MAX_AGE_SECONDS, AssetLeasingError::StalePrice);
```

Three checks, for three threat models:

- `publish_time <= now` — **No future-dated prices.** Guards against
  a malicious keeper manufacturing a "future" price to game the
  math. (In practice `publish_time` from Pyth is never future, but
  defence-in-depth.)
- `age <= 60 s` — **Not too old.** If Pyth has stalled (perhaps a
  degraded network), we refuse to liquidate rather than act on stale
  data.
- `price > 0` — `NonPositivePrice`. A zero or negative price would
  let a liquidator seize collateral for "free debt", obviously
  wrong.

### 6.5 The underwater check

Here is the actual inequality:

```
collateral × 10_000   <   debt × maintenance_margin_bps
^ left side             ^ right side
```

Rearranging: the position is underwater when the collateral-to-debt
ratio falls below the margin. Everything is in integers (Pyth's
`exponent` is folded into one side of the inequality to keep the
scales balanced). See the `is_underwater` function if you want to
walk through the math — it is well-commented.

---

## 7. Safety and edge cases worth knowing

A few corners of the design worth thinking about.

### 7.1 What if the lessee returns the tokens while liquidation is in flight?

Both `return_lease` and `liquidate` mutate the same `Lease` account.
Only one transaction per slot can win — Solana serialises mutations
to each account. Whichever lands first flips the status
(`Closed` for return, `Liquidated` for liquidate). The loser hits the
`constraint = lease.status == LeaseStatus::Active` check and fails.

So there is no "both happened" race. The user who reacts first wins.

### 7.2 What if the Pyth oracle stops updating?

The staleness check (`age <= 60 s`) simply fails. The `liquidate`
transaction aborts with `StalePrice`. Nothing bad happens — lessees
cannot be liquidated on stale data. The trade-off is that if prices
are unavailable and a lessee goes underwater, the lessor just has
to wait until prices are flowing again before a keeper can act.
That's usually preferable to the alternative.

### 7.3 What if the collateral mint is the same as the leased mint?

Nothing in `create_lease` currently forbids this. The handler would
succeed, and the resulting lease would be a "token A for token A"
rent — arguably pointless, but not dangerous. There is no
path where the two vaults would get mixed up (they are separate PDAs
with different seeds, even if they hold the same mint). If you
wanted to forbid it, a one-line `require!(leased_mint.key() !=
collateral_mint.key(), ...)` in `create_lease` would do it.

### 7.4 What if rent accrues for longer than the collateral can cover?

Great question. This is the normal case just before a liquidation:
rent has eaten through the collateral faster than the lessee has
topped it up.

The handler handles it gracefully:

```rust
let payable = rent_amount.min(context.accounts.collateral_amount_available());
```

Rent is **capped at the vault balance**. The program never tries to
move tokens that aren't there. The unpaid rent becomes implicit debt,
which will trigger liquidation (or default recovery on `end_ts`)
because the position will fail the maintenance-margin check. In
other words: "lessee ran out of money" and "lessee is underwater" end
up in the same place. The math lines up.

### 7.5 What if `end_ts` is reached but nobody calls anything?

Nothing special happens automatically. `Active` leases continue to
exist forever until somebody calls `return_lease`, `liquidate`, or
`close_expired`. Rent stops accruing at `end_ts` because of the
`cutoff = now.min(end_ts)` clamp, so the lessor is not building up
infinite phantom debt. It is just a lease sitting in state waiting
to be cleaned up.

In production you would usually run a small keeper that calls
`close_expired` shortly after `end_ts` on any `Active` lease, just
to reclaim the rent-exempt lamports and tidy up the account.

### 7.6 Who pays for what?

- **Lessor** pays for creating the `Lease`, the `leased_vault` and
  the `collateral_vault` at listing (rent-exempt lamports). They
  get those back on close, but also have to fund the lessor ATAs if
  they did not already exist.
- **Lessee** pays transaction fees for `take_lease`,
  `top_up_collateral` and `return_lease`, and funds their own ATA
  for the leased mint on `take_lease`. On `return_lease` they also
  fund the lessor's ATAs if these do not already exist.
- **Keeper** pays transaction fees for `liquidate`, and funds both
  the lessor's and their own collateral ATAs on first use. They
  get that back (and then some) via the bounty, so in practice the
  cost is negligible.
- **Anyone** who calls `pay_rent` pays a transaction fee. Usually
  this is the lessee or a keeper.

### 7.7 Bounty and max-margin caps

```rust
pub const MAX_MAINTENANCE_MARGIN_BPS: u16 = 50_000;   // 500%
pub const MAX_LIQUIDATION_BOUNTY_BPS: u16 = 2_000;    //  20%
```

These exist to prevent obvious griefing:

- A lessor cannot set a 10,000% margin and immediately liquidate
  their own lessee on day one.
- A lessor cannot set a 99% bounty that would funnel the lessee's
  entire collateral to a friendly keeper, netting Alice zero and
  pocketing the difference out of band. The 20% cap keeps the
  majority of the collateral with the actual victim (the lessor).

### 7.8 Anyone can call `pay_rent`, on purpose

This is deliberate. If only the lessee could pay rent, a lazy (or
absent) lessee could let rent pile up implicitly until they are
liquidated — and the *accrued but unpaid* rent would still be
unpaid in the rent cap path. By letting keepers call `pay_rent`
before they call `liquidate`, they ensure the lessor has received
the owed rent first, regardless of the lessee's participation.

---

## 8. How to build and test

You need a working Solana + Anchor toolchain (`anchor --version`,
`solana --version` should both return something sensible). This
example was written against Anchor 1.0.

```bash
cd defi/asset-leasing/anchor
anchor build
cargo test
```

### What `anchor build` does

Compiles the on-chain program (`programs/asset-leasing`) down to a
Berkeley Packet Filter `.so` binary under `target/deploy/`. Generates
an IDL (interface description) plus Rust types the tests then use.

You have to run `anchor build` **before** `cargo test` — the tests
`include_bytes!` the compiled program so they can load it into
LiteSVM. If the `.so` does not exist, the tests won't compile.

### What `cargo test` does (and what LiteSVM is)

The tests live in `programs/asset-leasing/tests/test_asset_leasing.rs`.
They use **LiteSVM**, an in-memory Solana runtime. Think of it as a
"tiny local Solana" you spin up in milliseconds. No validator
process, no ledger on disk, no RPC round-trips. Each test gets a
fresh VM, mints its own tokens, deploys the program from the
`.so`, and runs a scenario.

LiteSVM is fast enough that you can test the full lifecycle —
`create → take → pay_rent → liquidate` — in a single `#[test]`,
advancing the clock sysvar by hand with `advance_clock_by`. The
existing tests cover:

| Test | Exercises |
| --- | --- |
| `create_lease_locks_tokens_and_lists` | Lessor funds the leased vault, lease account is created. |
| `take_lease_posts_collateral_and_delivers_tokens` | Collateral flows in, leased tokens flow out. |
| `pay_rent_streams_collateral_by_elapsed_time` | Rent math: elapsed × rate. |
| `top_up_collateral_increases_vault_balance` | Top-up actually increases the vault. |
| `return_lease_refunds_unused_collateral` | Happy path: rent paid, refund issued, accounts closed. |
| `liquidate_seizes_collateral_on_price_drop` | Mocked `PriceUpdateV2` that makes the position underwater. |
| `liquidate_rejects_healthy_position` | Fails with `PositionHealthy` when it would be wrong. |
| `close_expired_reclaims_collateral_after_end_ts` | Default recovery after `end_ts`. |
| `close_expired_cancels_listed_lease` | Lessor cancels an unclaimed listing. |

The tests do not pull in the real Pyth SDK — they synthesise a
`PriceUpdateV2` body with the right discriminator and offsets, set
the owner to the Pyth receiver program id, and install it into the
LiteSVM account store. This is the whole reason the program decodes
by hand rather than through an SDK — it lets the tests mock oracle
data without a network.

---

## 9. Extending this example

Real systems always need more features. Here are ideas a learner
can sink their teeth into, ordered roughly by difficulty.

### Easy

- **Add `require!` guards for bad combinations.** e.g. reject
  `create_lease` when `leased_mint == collateral_mint`, or when
  `rent_per_second × duration_seconds` would overflow a `u64`.
- **Emit events.** Add Anchor `#[event]`s on lease creation,
  take-up, liquidation — a UI can then subscribe without polling.

### Medium

- **Variable rent based on utilisation.** Instead of a fixed
  `rent_per_second`, let the protocol compute rent from a curve:
  e.g. more expensive when most of a lessor's inventory is
  currently leased. This mimics how lending protocols price
  borrow rates.
- **Whitelisted lessees.** A `whitelist` account that maps
  `(lessor, lessee) -> allowed`. `take_lease` requires the entry
  to exist. Basis for a KYC-gated product.
- **Protocol fee.** A small cut of every `pay_rent` goes to a
  treasury account the program derives. Same shape as the keeper
  bounty, but with a different destination.
- **Partial returns.** Allow the lessee to return part of the
  leased tokens and get a proportional share of collateral back.
  Tricky: you need to also decide whether the rent rate scales
  with the reduced amount.

### Harder

- **Multi-token collateral.** The lessee posts a basket (e.g. 40%
  SOL, 60% USDC) instead of a single mint. `is_underwater` now
  sums each bucket's value using its own Pyth feed.
- **Pin a Pyth feed to a lease.** Instead of trusting the keeper to
  supply a correct feed, store a `price_feed_id` on the `Lease`
  and require the `PriceUpdateV2`'s `feed_id` field to match. Closes
  the "wrong feed" loophole.
- **Dutch auction liquidation.** Instead of a fixed bounty,
  the liquidation price starts steep and decays over time. Whoever
  is willing to pay the least takes the trade. Better price
  discovery, more complex bookkeeping.

Pick one, try it, and run the existing tests plus a new one for
your feature. That is the honest way to learn this stuff.

---

## 10. Further reading

### Solana + Anchor

- [Anchor Book](https://www.anchor-lang.com/) — the official guide,
  especially the chapters on PDAs, CPIs and account constraints.
- [Anchor 1.0 release notes](https://github.com/coral-xyz/anchor/releases)
  — what changed versus 0.30.
- [SPL Token program docs](https://spl.solana.com/token) — the
  token mint / account / transfer model this example builds on.
- [Solana Cookbook — PDAs](https://solanacookbook.com/core-concepts/pdas.html)
  — if §2.9 felt too short, start here.

### Pyth

- [Pyth Network docs](https://docs.pyth.network/) — what a price
  feed is, what publishers look like.
- [Pyth Solana Receiver](https://docs.pyth.network/price-feeds/use-real-time-data/solana)
  — how the on-chain `PriceUpdateV2` accounts are produced.
- [Anchor / Pyth borsh conflict tracking issue](https://github.com/pyth-network/pyth-crosschain/issues)
  — watch this if you want to eventually drop the manual decode.

### Sibling examples in this repo

- [`tokens/escrow/anchor`](../../../tokens/escrow/anchor) — the
  simplest "lock tokens until a condition is met" Anchor program.
  Good warm-up if the PDA / vault pattern here felt new.
- [`defi/clob/anchor`](../../clob/anchor) — an on-chain central
  limit order book. Order matching instead of lease lifecycle.
- Hunt around in `defi/` and `tokens/` — each folder's README is
  a little self-contained tutorial.

### Finance concepts, for the curious

- Investopedia on **maintenance margin**, **basis points** and
  **liquidation** — plain English definitions.
- Aave / Compound technical papers — production lending protocols
  use the same collateral-and-liquidation vocabulary. Reading
  theirs after this will feel familiar.

---

_This README is a teaching document. If any claim about the program
contradicts what the code actually does, file an issue — the code is
the source of truth._
