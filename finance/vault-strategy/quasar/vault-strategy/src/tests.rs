//! quasar-test integration tests. `strategy_setup` drives the manager-side
//! setup (registry, approve asset, strategy, add asset) and asserts state.
//! The deposit tests are two-program tests: they load the mock swap router
//! too, wire up rates and a Pyth-shaped price feed, and deposit, checking that
//! the first deposit is priced by the virtual-share offset (one whole share per
//! USDC) and deployed into the basket through the router CPI, and that a
//! donation straight into the USDC vault cannot floor the next deposit to zero
//! shares.

use {
    crate::{
        cpi::{
            AddAssetInstruction, ApproveAssetInstruction, DepositInstruction,
            InitializeRegistryInstruction, InitializeStrategyInstruction, WithdrawInstruction,
        },
        state::{
            AssetConfig, AssetVaultPda, Registry, ShareMintPda, Strategy, UsdcVaultPda,
            SHARE_DECIMALS, VIRTUAL_SHARES,
        },
    },
    quasar_test::prelude::*,
};

const DECIMALS: u8 = 6;
const FEE_BPS: u16 = 100;
const MAX_SLIPPAGE_BPS: u16 = 100;

// Router program (loaded for the deposit tests).
const ROUTER_ID_STR: &str = "SWPR8Rk3aq3DrDGLdaANq7xCMnXoUFUJWJJmCWxc8Jm";
const RATE: u64 = 250; // USDC base units per asset base unit
const NOW: i64 = 1_000; // fixed clock for the deposit tests
const STRATEGY_INDEX: u64 = 0;

// Deterministic addresses.
const AUTHORITY: Pubkey = Pubkey::new_from_array([1; 32]);
const MANAGER: Pubkey = Pubkey::new_from_array([2; 32]);
const DEPOSITOR: Pubkey = Pubkey::new_from_array([3; 32]);
const USDC_MINT: Pubkey = Pubkey::new_from_array([4; 32]);
const ASSET_MINT: Pubkey = Pubkey::new_from_array([5; 32]);
const PRICE_FEED: Pubkey = Pubkey::new_from_array([6; 32]);
const DEPOSITOR_USDC: Pubkey = Pubkey::new_from_array([7; 32]);
const DEPOSITOR_SHARE: Pubkey = Pubkey::new_from_array([8; 32]);
const FEED_OWNER: Pubkey = Pubkey::new_from_array([9; 32]);
const DEPOSITOR_ASSET: Pubkey = Pubkey::new_from_array([10; 32]);
const ATTACKER: Pubkey = Pubkey::new_from_array([11; 32]);
const ATTACKER_USDC: Pubkey = Pubkey::new_from_array([12; 32]);
const ATTACKER_SHARE: Pubkey = Pubkey::new_from_array([13; 32]);
const ATTACKER_ASSET: Pubkey = Pubkey::new_from_array([14; 32]);

fn router_id() -> Pubkey {
    ROUTER_ID_STR.parse().unwrap()
}

// Router PDAs (owned by the router program, so derived manually).
fn router_config_pda() -> Pubkey {
    Pubkey::find_program_address(&[b"router_config"], &router_id()).0
}
fn router_authority_pda() -> Pubkey {
    Pubkey::find_program_address(&[b"router_authority"], &router_id()).0
}
fn router_treasury_pda() -> Pubkey {
    Pubkey::find_program_address(&[b"treasury"], &router_id()).0
}
fn router_rate_pda(mint: &Pubkey) -> Pubkey {
    Pubkey::find_program_address(&[b"rate", mint.as_ref()], &router_id()).0
}

// A Pyth PriceUpdateV2-shaped account: `price` (i64) at offset 73,
// `publish_time` (i64) at offset 93. The program reads only those two fields.
fn add_pyth_feed(test: &mut Test, price: i64, publish_time: i64) {
    let mut data = vec![0u8; 200];
    data[73..81].copy_from_slice(&price.to_le_bytes());
    data[93..101].copy_from_slice(&publish_time.to_le_bytes());
    test.set_account(Account::new(PRICE_FEED, FEED_OWNER, 1_000_000, data));
}

/// The strategy-side PDAs the assertions read.
struct Pdas {
    strategy: Pubkey,
    asset_config: Pubkey,
    vault_asset: Pubkey,
    vault_usdc: Pubkey,
    share_mint: Pubkey,
}

fn pdas(test: &Test) -> Pdas {
    let strategy = test.derive_pda(Strategy::seeds(STRATEGY_INDEX));
    Pdas {
        strategy,
        asset_config: test.derive_pda(AssetConfig::seeds(&strategy, 0)),
        vault_asset: test.derive_pda(AssetVaultPda::seeds(&strategy, 0)),
        vault_usdc: test.derive_pda(UsdcVaultPda::seeds(&strategy)),
        share_mint: test.derive_pda(ShareMintPda::seeds(&strategy)),
    }
}

/// Registry + approved asset + strategy + one basket asset at 100% weight.
fn setup_strategy(test: &mut Test, asset_mint_authority: Pubkey) {
    test.add(Wallet::new().at(AUTHORITY));
    test.add(Wallet::new().at(MANAGER));
    test.add(Mint::new(AUTHORITY).at(USDC_MINT).decimals(DECIMALS));
    test.add(
        Mint::new(asset_mint_authority)
            .at(ASSET_MINT)
            .decimals(DECIMALS),
    );

    let registry = test.derive_pda(Registry::seeds(&AUTHORITY));

    test.send(InitializeRegistryInstruction {
        authority: AUTHORITY,
    })
    .succeeds();
    test.send(ApproveAssetInstruction {
        authority: AUTHORITY,
        asset_mint: ASSET_MINT,
        price_feed: PRICE_FEED,
    })
    .succeeds();
    test.send(InitializeStrategyInstruction {
        manager: MANAGER,
        usdc_mint: USDC_MINT,
        registry,
        index: STRATEGY_INDEX,
        fee_bps: FEE_BPS,
        max_slippage_bps: MAX_SLIPPAGE_BPS,
        swap_router: router_id(),
    })
    .succeeds();
    test.send(AddAssetInstruction {
        manager: MANAGER,
        strategy_index_seed: STRATEGY_INDEX,
        registry,
        asset_mint: ASSET_MINT,
        strategy_asset_count_seed: 0,
        weight_bps: 10_000,
    })
    .succeeds();
}

#[quasar_test]
fn strategy_setup_records_the_basket(test: &mut Test) {
    setup_strategy(test, AUTHORITY);
    let w = pdas(test);

    let strategy = test.read::<Strategy>(w.strategy);
    assert_eq!(strategy.asset_count, 1, "asset_count");
    assert_eq!(
        u16::from(strategy.total_weight_bps),
        10_000,
        "total_weight_bps"
    );

    let asset_config = test.read::<AssetConfig>(w.asset_config);
    assert_eq!(u16::from(asset_config.weight_bps), 10_000, "weight_bps");
    assert_eq!(asset_config.mint, ASSET_MINT, "asset mint");
    assert_eq!(
        asset_config.price_feed, PRICE_FEED,
        "price feed copied from registry"
    );
}

/// Load the router program, set the clock, and build a single-asset strategy
/// whose asset the router mints, priced at 250 USDC per token on both the Pyth
/// feed and the router. Returns the strategy PDAs.
fn setup_router_and_strategy(test: &mut Test) -> Pdas {
    // Runtime read (NOT include_bytes!): quasar-test auto-loads only this
    // program's .so; the sibling router program is added explicitly.
    let router_elf =
        std::fs::read("../mock-swap-router/target/deploy/quasar_mock_swap_router.so").unwrap();
    test.add(Program::new(router_id(), &router_elf));
    test.warp_to_timestamp(NOW);

    let r_authority = router_authority_pda();
    // The asset mint is minted by the router authority.
    setup_strategy(test, r_authority);
    let w = pdas(test);

    // Asset priced 250 USDC/token: Pyth price = 250 * 10^8 so
    // asset_value = amount * price / 10^8 gives 250 USDC per token base unit.
    let pyth_price: i64 = 250 * 100_000_000;
    add_pyth_feed(test, pyth_price, NOW);

    // Initialize the router and set the asset's rate (hand-built: the router's
    // builders live in the sibling crate).
    let rent_id: Pubkey = "SysvarRent111111111111111111111111111111111"
        .parse()
        .unwrap();
    test.send(Instruction {
        program_id: router_id(),
        accounts: vec![
            AccountMeta::new(AUTHORITY, true),
            AccountMeta::new_readonly(USDC_MINT, false),
            AccountMeta::new(router_config_pda(), false),
            AccountMeta::new_readonly(rent_id, false),
            AccountMeta::new_readonly(SPL_TOKEN_PROGRAM_ID, false),
            AccountMeta::new_readonly(system_program::ID, false),
        ],
        data: vec![0u8],
    })
    .succeeds();
    let mut set_rate_data = vec![1u8];
    set_rate_data.extend_from_slice(&RATE.to_le_bytes());
    test.send(Instruction {
        program_id: router_id(),
        accounts: vec![
            AccountMeta::new(AUTHORITY, true),
            AccountMeta::new_readonly(router_config_pda(), false),
            AccountMeta::new_readonly(ASSET_MINT, false),
            AccountMeta::new_readonly(USDC_MINT, false),
            AccountMeta::new(router_rate_pda(&ASSET_MINT), false),
            AccountMeta::new_readonly(r_authority, false),
            AccountMeta::new(router_treasury_pda(), false),
            AccountMeta::new_readonly(rent_id, false),
            AccountMeta::new_readonly(SPL_TOKEN_PROGRAM_ID, false),
            AccountMeta::new_readonly(system_program::ID, false),
        ],
        data: set_rate_data,
    })
    .succeeds();
    w
}

/// A depositor's wallet plus their USDC, share, and asset token accounts (the
/// share account must exist before a deposit; the asset account before an
/// in-kind withdrawal).
fn add_depositor(test: &mut Test, w: &Pdas, owner: Pubkey, accounts: [Pubkey; 3], usdc: u64) {
    let [usdc_account, share_account, asset_account] = accounts;
    test.add(Wallet::new().at(owner));
    test.add(
        TokenAccount::new(USDC_MINT, owner)
            .at(usdc_account)
            .amount(usdc),
    );
    test.add(TokenAccount::new(w.share_mint, owner).at(share_account));
    test.add(TokenAccount::new(ASSET_MINT, owner).at(asset_account));
}

/// Deposit: declared accounts, then remaining accounts per basket asset
/// (asset_config, vault_asset, asset_mint, asset_rate, price_feed).
fn deposit(
    w: &Pdas,
    depositor: Pubkey,
    usdc_account: Pubkey,
    share_account: Pubkey,
    usdc_amount: u64,
    minimum_shares: u64,
) -> DepositInstruction {
    DepositInstruction {
        depositor,
        strategy_index_seed: STRATEGY_INDEX,
        usdc_mint: USDC_MINT,
        depositor_usdc_account: usdc_account,
        depositor_share_account: share_account,
        router_config: router_config_pda(),
        router_usdc_treasury: router_treasury_pda(),
        router_authority: router_authority_pda(),
        swap_router_program: router_id(),
        usdc_amount,
        minimum_shares,
        remaining_accounts: vec![
            AccountMeta::new_readonly(w.asset_config, false),
            AccountMeta::new(w.vault_asset, false),
            AccountMeta::new(ASSET_MINT, false),
            AccountMeta::new_readonly(router_rate_pda(&ASSET_MINT), false),
            AccountMeta::new_readonly(PRICE_FEED, false),
        ],
    }
}

/// Withdraw in kind: declared accounts, then remaining accounts per basket
/// asset (asset_config, vault_asset, asset_mint, user_asset_account).
fn withdraw(
    w: &Pdas,
    user: Pubkey,
    accounts: [Pubkey; 3],
    shares_to_burn: u64,
) -> WithdrawInstruction {
    let [usdc_account, share_account, asset_account] = accounts;
    WithdrawInstruction {
        user,
        strategy_index_seed: STRATEGY_INDEX,
        usdc_mint: USDC_MINT,
        user_share_account: share_account,
        user_usdc_account: usdc_account,
        shares_to_burn,
        min_usdc_out: 0,
        remaining_accounts: vec![
            AccountMeta::new_readonly(w.asset_config, false),
            AccountMeta::new(w.vault_asset, false),
            AccountMeta::new_readonly(ASSET_MINT, false),
            AccountMeta::new(asset_account, false),
        ],
    }
}

/// A holder's position valued in USDC minor units at the test's price: USDC
/// plus the asset at 250 USDC per token.
fn value_in_usdc(test: &Test, usdc_account: Pubkey, asset_account: Pubkey) -> u64 {
    test.tokens(usdc_account) + test.tokens(asset_account) * RATE
}

/// Two-program deposit: set up the router + a single-asset strategy, then
/// deposit USDC. The first deposit is priced by the virtual-share offset (1000
/// share minor units per USDC minor unit, one whole share per USDC) and deploys
/// the whole amount into the asset through the router CPI.
#[quasar_test]
fn deposit_mints_shares_and_deploys_into_the_basket(test: &mut Test) {
    let w = setup_router_and_strategy(test);

    const DEPOSIT: u64 = 1_000;
    const ASSET_OUT: u64 = DEPOSIT / RATE; // 4

    add_depositor(
        test,
        &w,
        DEPOSITOR,
        [DEPOSITOR_USDC, DEPOSITOR_SHARE, DEPOSITOR_ASSET],
        DEPOSIT,
    );

    test.send(deposit(
        &w,
        DEPOSITOR,
        DEPOSITOR_USDC,
        DEPOSITOR_SHARE,
        DEPOSIT,
        DEPOSIT * VIRTUAL_SHARES,
    ))
    .succeeds()
    // The first deposit mints VIRTUAL_SHARES share minor units per USDC minor
    // unit: 1000 USDC minor units become 1,000,000, a thousandth of a whole share.
    .has_tokens(DEPOSITOR_SHARE, DEPOSIT * VIRTUAL_SHARES)
    // The deposit was deployed into the asset via the router.
    .has_tokens(w.vault_asset, ASSET_OUT)
    .has_tokens(DEPOSITOR_USDC, 0)
    .has_tokens(router_treasury_pda(), DEPOSIT)
    // All USDC was swapped out of the vault into the asset.
    .has_tokens(w.vault_usdc, 0);

    // The share mint carries USDC's six decimals plus the virtual-share offset,
    // so those 1,000,000 minor units are a thousandth of a whole share. Token
    // mint layout: decimals is the byte at offset 44.
    let share_mint = test.account(w.share_mint).unwrap();
    assert_eq!(share_mint.data[44], SHARE_DECIMALS, "share mint decimals");
}

/// The first-depositor inflation attack: a dust deposit, then a donation straight
/// into the strategy's USDC vault, then a 1,000 USDC deposit with no
/// `minimum_shares` floor. The virtual offset prices the dust deposit at 1000
/// share minor units, splits the donation between those and the 1000 virtual
/// shares, and keeps the victim's shares nonzero: they redeem for all but a
/// fraction of a dollar, while the attacker recovers about half of the donation
/// and loses the rest to the virtual shares. Modeled on the lending example's
/// `raw_token_donation_does_not_inflate_exchange_rate`.
#[quasar_test]
fn donation_does_not_inflate_share_price(test: &mut Test) {
    let w = setup_router_and_strategy(test);

    const DONATION: u64 = 1_000_000_000; // 1,000 USDC
    const VICTIM_DEPOSIT: u64 = 1_000_000_000; // 1,000 USDC

    let attacker_accounts = [ATTACKER_USDC, ATTACKER_SHARE, ATTACKER_ASSET];
    let victim_accounts = [DEPOSITOR_USDC, DEPOSITOR_SHARE, DEPOSITOR_ASSET];
    add_depositor(test, &w, ATTACKER, attacker_accounts, 1 + DONATION);
    add_depositor(test, &w, DEPOSITOR, victim_accounts, VICTIM_DEPOSIT);

    // The attacker deposits one minor unit through the handler. The offset prices
    // it at 1 * (0 + 1000) / (0 + 1) = 1000 share minor units. The basket is one
    // asset at 100%, so the deploy leg swaps the minor unit through the router,
    // which at 250 USDC per token returns nothing for it: the vaults stay empty.
    test.send(deposit(&w, ATTACKER, ATTACKER_USDC, ATTACKER_SHARE, 1, 0))
        .succeeds()
        .has_tokens(ATTACKER_SHARE, VIRTUAL_SHARES)
        .has_tokens(w.vault_usdc, 0)
        .has_tokens(w.vault_asset, 0);

    // The attacker sends 1,000 USDC straight to the USDC vault with an ordinary
    // token transfer (SPL Token `Transfer`, instruction 3). The deposit handler
    // never ran, so the supply is still 1000 share minor units, now against a
    // NAV of 1,000 USDC.
    let mut transfer_data = vec![3u8];
    transfer_data.extend_from_slice(&DONATION.to_le_bytes());
    test.send(Instruction {
        program_id: SPL_TOKEN_PROGRAM_ID,
        accounts: vec![
            AccountMeta::new(ATTACKER_USDC, false),
            AccountMeta::new(w.vault_usdc, false),
            AccountMeta::new_readonly(ATTACKER, true),
        ],
        data: transfer_data,
    })
    .succeeds()
    .has_tokens(w.vault_usdc, DONATION);

    // The victim deposits 1,000 USDC with no minimum_shares floor. Without the
    // offset this would be 1,000,000,000 * 1 / 1,000,000,000 = 1 share minor unit,
    // a millionth of what they paid for. With it:
    // 1,000,000,000 * (1000 + 1000) / (1,000,000,000 + 1) = 1999 share minor units.
    const VICTIM_SHARES: u64 = 1_999;
    test.send(deposit(
        &w,
        DEPOSITOR,
        DEPOSITOR_USDC,
        DEPOSITOR_SHARE,
        VICTIM_DEPOSIT,
        0,
    ))
    .succeeds()
    .has_tokens(DEPOSITOR_SHARE, VICTIM_SHARES)
    // The victim's USDC was deployed into the asset: 1,000 USDC at 250 per token.
    .has_tokens(w.vault_asset, VICTIM_DEPOSIT / RATE);

    // The victim redeems everything in kind: 1999 of the 3999 effective shares
    // (1000 attacker + 1999 victim + 1000 virtual) of each vault, worth all but
    // about a quarter of a dollar of the 1,000 USDC they put in.
    test.send(withdraw(&w, DEPOSITOR, victim_accounts, VICTIM_SHARES))
        .succeeds()
        .has_tokens(DEPOSITOR_SHARE, 0);
    let victim_value = value_in_usdc(test, DEPOSITOR_USDC, DEPOSITOR_ASSET);
    assert!(victim_value <= VICTIM_DEPOSIT);
    let victim_loss = VICTIM_DEPOSIT - victim_value;
    assert!(
        victim_loss < 1_000_000,
        "victim lost {victim_loss} minor units, more than a dollar"
    );

    // The attacker redeems their 1000 share minor units: half of what is left,
    // the other half belonging to the virtual shares, which is to say to nobody.
    // They put in 1,000.000001 USDC through the handler and the donation together
    // and get back about 500 USDC, losing about a thousand times the victim's loss.
    test.send(withdraw(&w, ATTACKER, attacker_accounts, VIRTUAL_SHARES))
        .succeeds()
        .has_tokens(ATTACKER_SHARE, 0);
    let attacker_value = value_in_usdc(test, ATTACKER_USDC, ATTACKER_ASSET);
    let attacker_in = 1 + DONATION;
    assert!(
        attacker_value < attacker_in,
        "the attack must not pay: put in {attacker_in}, got back {attacker_value}"
    );
    let attacker_loss = attacker_in - attacker_value;
    assert!(
        attacker_loss >= VIRTUAL_SHARES * victim_loss,
        "attacker lost {attacker_loss}, victim lost {victim_loss}"
    );
    assert!(attacker_value <= attacker_in / 2 + 1_000_000);

    let strategy = test.read::<Strategy>(w.strategy);
    assert_eq!(u64::from(strategy.total_shares), 0, "total_shares");
}
