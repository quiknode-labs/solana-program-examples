# Asset Leasing Program

An on-chain program for renting and leasing digital assets (SPL tokens, NFTs) for a fixed duration on Solana.

## How It Works

```
┌─────────┐    list_asset     ┌──────────┐    rent_asset    ┌─────────┐
│  Owner   │ ───────────────▶ │  Vault   │ ──────────────▶  │ Renter  │
│ (token)  │                  │  (PDA)   │                  │ (token) │
└─────────┘                   └──────────┘                  └─────────┘
     ▲                             ▲                             │
     │     delist_asset            │      return_asset           │
     │◀────────────────────────────│◀────────────────────────────┘
     │ (only if no active lease)   │    (token back to vault)
     │                             │
     │                             │    claim_expired
     │◀────────────────────────────┘    (owner closes lease after expiry)
```

### Token Flow

1. **Owner** has an SPL token (fungible or NFT with amount=1)
2. **`list_asset`** — Owner deposits the token into a program-owned vault PDA and creates a `Listing` with pricing terms
3. **`rent_asset`** — Renter pays SOL (owner gets payment minus program fee), token moves from vault to renter
4. **`return_asset`** — Renter sends the token back to the vault, lease is marked returned
5. **`delist_asset`** — Owner withdraws the token from vault (only if no active lease)
6. **`claim_expired`** — Owner can close an expired lease if the renter hasn't returned the asset

### Program Fees

The program takes a configurable fee (default 2.5%) on each rental payment. Fees are paid directly to the authority in SOL during the `rent_asset` instruction. The authority can update the fee rate via `collect_fees`.

## State Accounts

| Account | Description |
|---------|-------------|
| `LeaseConfig` | Global config: authority address, fee basis points |
| `Listing` | Per-asset listing: owner, mint, price/second, duration bounds, active lease flag |
| `Lease` | Active lease: renter, listing reference, start/end time, returned flag |

## Instructions

| Instruction | Who | What |
|-------------|-----|------|
| `initialize` | Authority | Create the program config |
| `list_asset` | Asset owner | Deposit token to vault, create listing |
| `delist_asset` | Asset owner | Withdraw token from vault, close listing |
| `rent_asset` | Renter | Pay SOL, receive token, create lease |
| `return_asset` | Renter | Return token to vault, mark lease returned |
| `claim_expired` | Asset owner | Close expired lease, free listing |
| `collect_fees` | Authority | Update the program fee rate |

## Building

### Anchor

```bash
cd anchor
anchor build
cargo test --package asset-leasing  # LiteSVM integration tests
```

### Quasar

```bash
cd quasar
quasar build
cargo test
```

## Design Decisions

- **SOL payments** — Rental fees are paid in SOL via system program transfers (not SPL tokens) to keep the example focused. A production version might support stablecoin payments.
- **Price per second** — Flexible time-based pricing. The renter specifies a duration within the listing's min/max bounds.
- **No freeze authority** — The `claim_expired` instruction closes the lease state but cannot force-transfer the token back. In production, you'd use delegate/freeze authority or require collateral.
- **Token Interface** — Uses `token_interface` (not just `token`) to support both SPL Token and Token-2022 assets.
- **Quasar limitations** — Quasar is pre-release (0.0.0) and doesn't yet support cross-field references in PDA seed expressions. The Quasar version implements `initialize` and `collect_fees`; the full instruction set is in Anchor.

## Project Structure

```
defi/asset-leasing/
├── README.md
├── anchor/
│   ├── Anchor.toml
│   ├── Cargo.toml
│   ├── programs/asset-leasing/
│   │   ├── Cargo.toml
│   │   ├── src/
│   │   │   ├── lib.rs           # Program entrypoint
│   │   │   ├── state.rs         # LeaseConfig, Listing, Lease
│   │   │   ├── errors.rs        # Custom error codes
│   │   │   ├── constants.rs     # Seeds, fee defaults
│   │   │   └── instructions/    # One file per instruction
│   │   └── tests/
│   │       └── test.rs          # LiteSVM + solana-kite tests
│   └── tests-rs/
│       └── test.rs              # Copy for convention
└── quasar/
    ├── Cargo.toml
    ├── Quasar.toml
    └── src/
        ├── lib.rs               # Program with initialize + collect_fees
        ├── state.rs             # Account definitions
        ├── errors.rs            # Error codes
        ├── constants.rs         # Seeds, fee defaults
        └── tests.rs             # Test stubs
```
