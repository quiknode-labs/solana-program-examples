//! LiteSVM tests for the asset-leasing program.
//!
//! Covers the full lifecycle: listing, taking, rent streaming, top-ups,
//! early return, keeper liquidation via a mocked Pyth `PriceUpdateV2`
//! account, and lessor-initiated default recovery after expiry.

use {
    anchor_lang::{
        solana_program::{instruction::Instruction, pubkey::Pubkey, system_program},
        InstructionData, ToAccountMetas,
    },
    anchor_lang::solana_program::clock::Clock,
    litesvm::LiteSVM,
    solana_keypair::Keypair,
    solana_kite::{
        create_associated_token_account, create_token_mint, create_wallet,
        get_token_account_balance, mint_tokens_to_token_account,
        send_transaction_from_instructions,
    },
    solana_signer::Signer,
};

// Keep test-side seeds in sync with `programs/asset-leasing/src/constants.rs`.
// Duplicated rather than imported so tests stay self-contained.
const LEASE_SEED: &[u8] = b"lease";
const LEASED_VAULT_SEED: &[u8] = b"leased_vault";
const COLLATERAL_VAULT_SEED: &[u8] = b"collateral_vault";

// Pyth Receiver program id — matches `PYTH_RECEIVER_PROGRAM_ID` in the
// program. Kept as a &str so we can parse it once at the top of liquidation
// tests without pulling in extra crate types.
const PYTH_RECEIVER_PROGRAM_ID_STR: &str = "rec5EKMGg6MxZYaMdyBfgwp4d5rB9T1VQH5pJv5LtFJ";

// Matches `PRICE_UPDATE_V2_DISCRIMINATOR` in liquidate.rs — sha256 of
// "account:PriceUpdateV2" taken from the Pyth receiver IDL.
const PRICE_UPDATE_V2_DISCRIMINATOR: [u8; 8] = [34, 241, 35, 99, 157, 126, 244, 205];

fn token_program_id() -> Pubkey {
    "TokenkegQfeZyiNwAJbNbGKPFXCWuBvf9Ss623VQ5DA"
        .parse()
        .unwrap()
}

fn ata_program_id() -> Pubkey {
    "ATokenGPvbdGVxr1b2hvZbsiqW5xWH25efTNsLJA8knL"
        .parse()
        .unwrap()
}

fn derive_ata(wallet: &Pubkey, mint: &Pubkey) -> Pubkey {
    let (ata, _bump) = Pubkey::find_program_address(
        &[wallet.as_ref(), token_program_id().as_ref(), mint.as_ref()],
        &ata_program_id(),
    );
    ata
}

fn lease_pdas(program_id: &Pubkey, lessor: &Pubkey, lease_id: u64) -> (Pubkey, Pubkey, Pubkey) {
    let (lease, _) = Pubkey::find_program_address(
        &[LEASE_SEED, lessor.as_ref(), &lease_id.to_le_bytes()],
        program_id,
    );
    let (leased_vault, _) =
        Pubkey::find_program_address(&[LEASED_VAULT_SEED, lease.as_ref()], program_id);
    let (collateral_vault, _) =
        Pubkey::find_program_address(&[COLLATERAL_VAULT_SEED, lease.as_ref()], program_id);
    (lease, leased_vault, collateral_vault)
}

struct Scenario {
    svm: LiteSVM,
    program_id: Pubkey,
    // `payer` funds the mint authority + ATA creations during setup but is
    // not used directly by the tests afterwards.
    #[allow(dead_code)]
    payer: Keypair,
    lessor: Keypair,
    lessee: Keypair,
    keeper: Keypair,
    leased_mint: Pubkey,
    collateral_mint: Pubkey,
    lessor_leased_ata: Pubkey,
    lessee_collateral_ata: Pubkey,
}

fn full_setup() -> Scenario {
    let program_id = asset_leasing::id();
    let mut svm = LiteSVM::new();
    let program_bytes = include_bytes!("../../../target/deploy/asset_leasing.so");
    svm.add_program(program_id, program_bytes).unwrap();

    let payer = create_wallet(&mut svm, 100_000_000_000).unwrap();
    let lessor = create_wallet(&mut svm, 10_000_000_000).unwrap();
    let lessee = create_wallet(&mut svm, 10_000_000_000).unwrap();
    let keeper = create_wallet(&mut svm, 10_000_000_000).unwrap();

    // 6 decimals matches USDC and keeps test arithmetic readable.
    let decimals = 6u8;
    let leased_mint = create_token_mint(&mut svm, &payer, decimals, None).unwrap();
    let collateral_mint = create_token_mint(&mut svm, &payer, decimals, None).unwrap();

    let lessor_leased_ata =
        create_associated_token_account(&mut svm, &lessor.pubkey(), &leased_mint, &payer).unwrap();
    mint_tokens_to_token_account(
        &mut svm,
        &leased_mint,
        &lessor_leased_ata,
        1_000_000_000,
        &payer,
    )
    .unwrap();

    let lessee_collateral_ata =
        create_associated_token_account(&mut svm, &lessee.pubkey(), &collateral_mint, &payer)
            .unwrap();
    mint_tokens_to_token_account(
        &mut svm,
        &collateral_mint,
        &lessee_collateral_ata,
        1_000_000_000,
        &payer,
    )
    .unwrap();

    // Anchor macros init the Lease + vault accounts — LiteSVM's default clock
    // is epoch 0 which makes the first `take_lease` have start_ts=0 and look
    // identical to a Listed lease. Advance once so rent math has signal.
    advance_clock_to(&mut svm, 1_700_000_000);

    Scenario {
        svm,
        program_id,
        payer,
        lessor,
        lessee,
        keeper,
        leased_mint,
        collateral_mint,
        lessor_leased_ata,
        lessee_collateral_ata,
    }
}

fn advance_clock_to(svm: &mut LiteSVM, unix_timestamp: i64) {
    let mut clock = svm.get_sysvar::<Clock>();
    clock.unix_timestamp = unix_timestamp;
    svm.set_sysvar::<Clock>(&clock);
}

fn advance_clock_by(svm: &mut LiteSVM, delta_seconds: i64) {
    let mut clock = svm.get_sysvar::<Clock>();
    clock.unix_timestamp += delta_seconds;
    svm.set_sysvar::<Clock>(&clock);
}

fn current_clock(svm: &LiteSVM) -> i64 {
    svm.get_sysvar::<Clock>().unix_timestamp
}

// ---------------------------------------------------------------------------
// Instruction builders
// ---------------------------------------------------------------------------

#[allow(clippy::too_many_arguments)]
fn build_create_lease_ix(
    sc: &Scenario,
    lease_id: u64,
    leased_amount: u64,
    required_collateral_amount: u64,
    rent_per_second: u64,
    duration_seconds: i64,
    maintenance_margin_bps: u16,
    liquidation_bounty_bps: u16,
    feed_id: [u8; 32],
) -> Instruction {
    let (lease, leased_vault, collateral_vault) =
        lease_pdas(&sc.program_id, &sc.lessor.pubkey(), lease_id);
    Instruction::new_with_bytes(
        sc.program_id,
        &asset_leasing::instruction::CreateLease {
            lease_id,
            leased_amount,
            required_collateral_amount,
            rent_per_second,
            duration_seconds,
            maintenance_margin_bps,
            liquidation_bounty_bps,
            feed_id,
        }
        .data(),
        asset_leasing::accounts::CreateLease {
            lessor: sc.lessor.pubkey(),
            leased_mint: sc.leased_mint,
            collateral_mint: sc.collateral_mint,
            lessor_leased_account: sc.lessor_leased_ata,
            lease,
            leased_vault,
            collateral_vault,
            token_program: token_program_id(),
            system_program: system_program::id(),
        }
        .to_account_metas(None),
    )
}

fn build_take_lease_ix(sc: &Scenario, lease_id: u64) -> Instruction {
    let (lease, leased_vault, collateral_vault) =
        lease_pdas(&sc.program_id, &sc.lessor.pubkey(), lease_id);
    let lessee_leased_ata = derive_ata(&sc.lessee.pubkey(), &sc.leased_mint);
    Instruction::new_with_bytes(
        sc.program_id,
        &asset_leasing::instruction::TakeLease {}.data(),
        asset_leasing::accounts::TakeLease {
            lessee: sc.lessee.pubkey(),
            lessor: sc.lessor.pubkey(),
            lease,
            leased_mint: sc.leased_mint,
            collateral_mint: sc.collateral_mint,
            leased_vault,
            collateral_vault,
            lessee_collateral_account: sc.lessee_collateral_ata,
            lessee_leased_account: lessee_leased_ata,
            token_program: token_program_id(),
            associated_token_program: ata_program_id(),
            system_program: system_program::id(),
        }
        .to_account_metas(None),
    )
}

fn build_pay_rent_ix(sc: &Scenario, lease_id: u64) -> Instruction {
    let (lease, _leased_vault, collateral_vault) =
        lease_pdas(&sc.program_id, &sc.lessor.pubkey(), lease_id);
    let lessor_collateral_ata = derive_ata(&sc.lessor.pubkey(), &sc.collateral_mint);
    Instruction::new_with_bytes(
        sc.program_id,
        &asset_leasing::instruction::PayRent {}.data(),
        asset_leasing::accounts::PayRent {
            payer: sc.lessee.pubkey(),
            lessor: sc.lessor.pubkey(),
            lease,
            collateral_mint: sc.collateral_mint,
            collateral_vault,
            lessor_collateral_account: lessor_collateral_ata,
            token_program: token_program_id(),
            associated_token_program: ata_program_id(),
            system_program: system_program::id(),
        }
        .to_account_metas(None),
    )
}

fn build_top_up_ix(sc: &Scenario, lease_id: u64, amount: u64) -> Instruction {
    let (lease, _leased_vault, collateral_vault) =
        lease_pdas(&sc.program_id, &sc.lessor.pubkey(), lease_id);
    Instruction::new_with_bytes(
        sc.program_id,
        &asset_leasing::instruction::TopUpCollateral { amount }.data(),
        asset_leasing::accounts::TopUpCollateral {
            lessee: sc.lessee.pubkey(),
            lessor: sc.lessor.pubkey(),
            lease,
            collateral_mint: sc.collateral_mint,
            collateral_vault,
            lessee_collateral_account: sc.lessee_collateral_ata,
            token_program: token_program_id(),
        }
        .to_account_metas(None),
    )
}

fn build_return_lease_ix(sc: &Scenario, lease_id: u64) -> Instruction {
    let (lease, leased_vault, collateral_vault) =
        lease_pdas(&sc.program_id, &sc.lessor.pubkey(), lease_id);
    let lessee_leased_ata = derive_ata(&sc.lessee.pubkey(), &sc.leased_mint);
    let lessor_collateral_ata = derive_ata(&sc.lessor.pubkey(), &sc.collateral_mint);
    Instruction::new_with_bytes(
        sc.program_id,
        &asset_leasing::instruction::ReturnLease {}.data(),
        asset_leasing::accounts::ReturnLease {
            lessee: sc.lessee.pubkey(),
            lessor: sc.lessor.pubkey(),
            lease,
            leased_mint: sc.leased_mint,
            collateral_mint: sc.collateral_mint,
            leased_vault,
            collateral_vault,
            lessee_leased_account: lessee_leased_ata,
            lessee_collateral_account: sc.lessee_collateral_ata,
            lessor_leased_account: sc.lessor_leased_ata,
            lessor_collateral_account: lessor_collateral_ata,
            token_program: token_program_id(),
            associated_token_program: ata_program_id(),
            system_program: system_program::id(),
        }
        .to_account_metas(None),
    )
}

fn build_liquidate_ix(sc: &Scenario, lease_id: u64, price_update: Pubkey) -> Instruction {
    let (lease, leased_vault, collateral_vault) =
        lease_pdas(&sc.program_id, &sc.lessor.pubkey(), lease_id);
    let lessor_collateral_ata = derive_ata(&sc.lessor.pubkey(), &sc.collateral_mint);
    let keeper_collateral_ata = derive_ata(&sc.keeper.pubkey(), &sc.collateral_mint);
    Instruction::new_with_bytes(
        sc.program_id,
        &asset_leasing::instruction::Liquidate {}.data(),
        asset_leasing::accounts::Liquidate {
            keeper: sc.keeper.pubkey(),
            lessor: sc.lessor.pubkey(),
            lease,
            leased_mint: sc.leased_mint,
            collateral_mint: sc.collateral_mint,
            leased_vault,
            collateral_vault,
            lessor_collateral_account: lessor_collateral_ata,
            keeper_collateral_account: keeper_collateral_ata,
            price_update,
            token_program: token_program_id(),
            associated_token_program: ata_program_id(),
            system_program: system_program::id(),
        }
        .to_account_metas(None),
    )
}

fn build_close_expired_ix(sc: &Scenario, lease_id: u64) -> Instruction {
    let (lease, leased_vault, collateral_vault) =
        lease_pdas(&sc.program_id, &sc.lessor.pubkey(), lease_id);
    let lessor_collateral_ata = derive_ata(&sc.lessor.pubkey(), &sc.collateral_mint);
    Instruction::new_with_bytes(
        sc.program_id,
        &asset_leasing::instruction::CloseExpired {}.data(),
        asset_leasing::accounts::CloseExpired {
            lessor: sc.lessor.pubkey(),
            lease,
            leased_mint: sc.leased_mint,
            collateral_mint: sc.collateral_mint,
            leased_vault,
            collateral_vault,
            lessor_leased_account: sc.lessor_leased_ata,
            lessor_collateral_account: lessor_collateral_ata,
            token_program: token_program_id(),
            associated_token_program: ata_program_id(),
            system_program: system_program::id(),
        }
        .to_account_metas(None),
    )
}

/// Build a minimal `PriceUpdateV2` account body with the requested price and
/// exponent, timestamped `publish_time`. Fields not used by the program are
/// filled with zero bytes.
fn build_price_update_data(
    feed_id: [u8; 32],
    price: i64,
    exponent: i32,
    publish_time: i64,
) -> Vec<u8> {
    // Size layout:
    // 8 disc + 32 write_authority + 1 verification_level + 32 feed_id +
    // 8 price + 8 conf + 4 exponent + 8 publish_time + 8 prev_publish_time +
    // 8 ema_price + 8 ema_conf + 8 posted_slot = 141 bytes.
    const TOTAL_LEN: usize = 141;
    let mut data = Vec::with_capacity(TOTAL_LEN);
    data.extend_from_slice(&PRICE_UPDATE_V2_DISCRIMINATOR);
    // write_authority — irrelevant for liquidation logic.
    data.extend_from_slice(&[0u8; 32]);
    // verification_level = Full (1).
    data.push(1);
    data.extend_from_slice(&feed_id);
    data.extend_from_slice(&price.to_le_bytes());
    data.extend_from_slice(&0u64.to_le_bytes()); // conf
    data.extend_from_slice(&exponent.to_le_bytes());
    data.extend_from_slice(&publish_time.to_le_bytes());
    data.extend_from_slice(&publish_time.to_le_bytes()); // prev_publish_time
    data.extend_from_slice(&0i64.to_le_bytes()); // ema_price
    data.extend_from_slice(&0u64.to_le_bytes()); // ema_conf
    data.extend_from_slice(&0u64.to_le_bytes()); // posted_slot
    data
}

/// Install a mock `PriceUpdateV2` account owned by the Pyth receiver program.
fn mock_price_update(
    svm: &mut LiteSVM,
    address: Pubkey,
    feed_id: [u8; 32],
    price: i64,
    exponent: i32,
    publish_time: i64,
) {
    let data = build_price_update_data(feed_id, price, exponent, publish_time);
    let lamports = svm.minimum_balance_for_rent_exemption(data.len());
    let owner: Pubkey = PYTH_RECEIVER_PROGRAM_ID_STR.parse().unwrap();
    svm.set_account(
        address,
        solana_account::Account {
            lamports,
            data,
            owner,
            executable: false,
            rent_epoch: 0,
        },
    )
    .unwrap();
}

// ---------------------------------------------------------------------------
// Tests
// ---------------------------------------------------------------------------

// Shared lease parameters so the sanity assertions line up across tests.
const LEASED_AMOUNT: u64 = 100_000_000; // 100 "leased" tokens (6 dp)
const REQUIRED_COLLATERAL: u64 = 200_000_000; // 200 collateral tokens
const RENT_PER_SECOND: u64 = 10; // 10 base-units / sec
const DURATION_SECONDS: i64 = 60 * 60 * 24; // 24h
const MAINTENANCE_MARGIN_BPS: u16 = 12_000; // 120%
const LIQUIDATION_BOUNTY_BPS: u16 = 500; // 5%
// Arbitrary 32-byte Pyth feed id the tests pin their leases to. The
// mocked `PriceUpdateV2` accounts carry the same id so the feed-pinning
// check in liquidate passes. `liquidate_rejects_mismatched_price_feed`
// flips one byte of this to prove the check rejects foreign feeds.
const FEED_ID: [u8; 32] = [0xAB; 32];

#[test]
fn create_lease_locks_tokens_and_lists() {
    let mut sc = full_setup();

    let lease_id = 1u64;
    let ix = build_create_lease_ix(
        &sc,
        lease_id,
        LEASED_AMOUNT,
        REQUIRED_COLLATERAL,
        RENT_PER_SECOND,
        DURATION_SECONDS,
        MAINTENANCE_MARGIN_BPS,
        LIQUIDATION_BOUNTY_BPS,
        FEED_ID,
    );
    send_transaction_from_instructions(&mut sc.svm, vec![ix], &[&sc.lessor], &sc.lessor.pubkey())
        .unwrap();

    let (lease_pda, leased_vault, collateral_vault) =
        lease_pdas(&sc.program_id, &sc.lessor.pubkey(), lease_id);

    // Leased tokens escrowed.
    assert_eq!(
        get_token_account_balance(&sc.svm, &leased_vault).unwrap(),
        LEASED_AMOUNT
    );
    // Collateral vault exists but has no collateral yet.
    assert_eq!(
        get_token_account_balance(&sc.svm, &collateral_vault).unwrap(),
        0
    );
    // Lessor's leased balance dropped by the escrowed amount.
    assert_eq!(
        get_token_account_balance(&sc.svm, &sc.lessor_leased_ata).unwrap(),
        1_000_000_000 - LEASED_AMOUNT
    );

    // Lease account exists and is owned by the program.
    let lease_account = sc.svm.get_account(&lease_pda).expect("lease PDA missing");
    assert_eq!(lease_account.owner, sc.program_id);
    assert!(!lease_account.data.is_empty());
}

#[test]
fn take_lease_posts_collateral_and_delivers_tokens() {
    let mut sc = full_setup();
    let lease_id = 2u64;

    let create_ix = build_create_lease_ix(
        &sc,
        lease_id,
        LEASED_AMOUNT,
        REQUIRED_COLLATERAL,
        RENT_PER_SECOND,
        DURATION_SECONDS,
        MAINTENANCE_MARGIN_BPS,
        LIQUIDATION_BOUNTY_BPS,
        FEED_ID,
    );
    send_transaction_from_instructions(
        &mut sc.svm,
        vec![create_ix],
        &[&sc.lessor],
        &sc.lessor.pubkey(),
    )
    .unwrap();

    let take_ix = build_take_lease_ix(&sc, lease_id);
    send_transaction_from_instructions(
        &mut sc.svm,
        vec![take_ix],
        &[&sc.lessee],
        &sc.lessee.pubkey(),
    )
    .unwrap();

    let (_, leased_vault, collateral_vault) =
        lease_pdas(&sc.program_id, &sc.lessor.pubkey(), lease_id);
    let lessee_leased_ata = derive_ata(&sc.lessee.pubkey(), &sc.leased_mint);

    // Leased vault drained into the lessee.
    assert_eq!(get_token_account_balance(&sc.svm, &leased_vault).unwrap(), 0);
    assert_eq!(
        get_token_account_balance(&sc.svm, &lessee_leased_ata).unwrap(),
        LEASED_AMOUNT
    );
    // Collateral moved from the lessee into the collateral vault.
    assert_eq!(
        get_token_account_balance(&sc.svm, &collateral_vault).unwrap(),
        REQUIRED_COLLATERAL
    );
    assert_eq!(
        get_token_account_balance(&sc.svm, &sc.lessee_collateral_ata).unwrap(),
        1_000_000_000 - REQUIRED_COLLATERAL
    );
}

#[test]
fn pay_rent_streams_collateral_by_elapsed_time() {
    let mut sc = full_setup();
    let lease_id = 3u64;

    let create_ix = build_create_lease_ix(
        &sc,
        lease_id,
        LEASED_AMOUNT,
        REQUIRED_COLLATERAL,
        RENT_PER_SECOND,
        DURATION_SECONDS,
        MAINTENANCE_MARGIN_BPS,
        LIQUIDATION_BOUNTY_BPS,
        FEED_ID,
    );
    let take_ix = build_take_lease_ix(&sc, lease_id);
    send_transaction_from_instructions(
        &mut sc.svm,
        vec![create_ix, take_ix],
        &[&sc.lessor, &sc.lessee],
        &sc.lessor.pubkey(),
    )
    .unwrap();

    let elapsed: i64 = 120; // 2 minutes
    advance_clock_by(&mut sc.svm, elapsed);

    let pay_ix = build_pay_rent_ix(&sc, lease_id);
    send_transaction_from_instructions(
        &mut sc.svm,
        vec![pay_ix],
        &[&sc.lessee],
        &sc.lessee.pubkey(),
    )
    .unwrap();

    let expected_rent = (elapsed as u64) * RENT_PER_SECOND;
    let lessor_collateral_ata = derive_ata(&sc.lessor.pubkey(), &sc.collateral_mint);
    assert_eq!(
        get_token_account_balance(&sc.svm, &lessor_collateral_ata).unwrap(),
        expected_rent
    );
    let (_, _, collateral_vault) = lease_pdas(&sc.program_id, &sc.lessor.pubkey(), lease_id);
    assert_eq!(
        get_token_account_balance(&sc.svm, &collateral_vault).unwrap(),
        REQUIRED_COLLATERAL - expected_rent
    );
}

#[test]
fn top_up_collateral_increases_vault_balance() {
    let mut sc = full_setup();
    let lease_id = 4u64;

    let create_ix = build_create_lease_ix(
        &sc,
        lease_id,
        LEASED_AMOUNT,
        REQUIRED_COLLATERAL,
        RENT_PER_SECOND,
        DURATION_SECONDS,
        MAINTENANCE_MARGIN_BPS,
        LIQUIDATION_BOUNTY_BPS,
        FEED_ID,
    );
    let take_ix = build_take_lease_ix(&sc, lease_id);
    send_transaction_from_instructions(
        &mut sc.svm,
        vec![create_ix, take_ix],
        &[&sc.lessor, &sc.lessee],
        &sc.lessor.pubkey(),
    )
    .unwrap();

    let top_up_amount: u64 = 50_000_000;
    let top_up_ix = build_top_up_ix(&sc, lease_id, top_up_amount);
    send_transaction_from_instructions(
        &mut sc.svm,
        vec![top_up_ix],
        &[&sc.lessee],
        &sc.lessee.pubkey(),
    )
    .unwrap();

    let (_, _, collateral_vault) = lease_pdas(&sc.program_id, &sc.lessor.pubkey(), lease_id);
    assert_eq!(
        get_token_account_balance(&sc.svm, &collateral_vault).unwrap(),
        REQUIRED_COLLATERAL + top_up_amount
    );
}

#[test]
fn return_lease_refunds_unused_collateral() {
    let mut sc = full_setup();
    let lease_id = 5u64;

    let create_ix = build_create_lease_ix(
        &sc,
        lease_id,
        LEASED_AMOUNT,
        REQUIRED_COLLATERAL,
        RENT_PER_SECOND,
        DURATION_SECONDS,
        MAINTENANCE_MARGIN_BPS,
        LIQUIDATION_BOUNTY_BPS,
        FEED_ID,
    );
    let take_ix = build_take_lease_ix(&sc, lease_id);
    send_transaction_from_instructions(
        &mut sc.svm,
        vec![create_ix, take_ix],
        &[&sc.lessor, &sc.lessee],
        &sc.lessor.pubkey(),
    )
    .unwrap();

    // Lessee returns early — 10 minutes in, for a 24h lease.
    let elapsed: i64 = 600;
    advance_clock_by(&mut sc.svm, elapsed);

    let return_ix = build_return_lease_ix(&sc, lease_id);
    send_transaction_from_instructions(
        &mut sc.svm,
        vec![return_ix],
        &[&sc.lessee],
        &sc.lessee.pubkey(),
    )
    .unwrap();

    let rent_paid = (elapsed as u64) * RENT_PER_SECOND;
    let refund_expected = REQUIRED_COLLATERAL - rent_paid;

    // Lessor got their leased tokens back.
    assert_eq!(
        get_token_account_balance(&sc.svm, &sc.lessor_leased_ata).unwrap(),
        1_000_000_000
    );
    // Lessor also received the accrued rent.
    let lessor_collateral_ata = derive_ata(&sc.lessor.pubkey(), &sc.collateral_mint);
    assert_eq!(
        get_token_account_balance(&sc.svm, &lessor_collateral_ata).unwrap(),
        rent_paid
    );
    // Lessee got the unused-time portion of their collateral back.
    assert_eq!(
        get_token_account_balance(&sc.svm, &sc.lessee_collateral_ata).unwrap(),
        1_000_000_000 - REQUIRED_COLLATERAL + refund_expected
    );

    // Lease + vault PDAs are closed.
    let (lease_pda, leased_vault, collateral_vault) =
        lease_pdas(&sc.program_id, &sc.lessor.pubkey(), lease_id);
    assert!(sc.svm.get_account(&lease_pda).is_none());
    assert!(sc.svm.get_account(&leased_vault).is_none());
    assert!(sc.svm.get_account(&collateral_vault).is_none());
}

#[test]
fn liquidate_seizes_collateral_on_price_drop() {
    let mut sc = full_setup();
    let lease_id = 6u64;

    let create_ix = build_create_lease_ix(
        &sc,
        lease_id,
        LEASED_AMOUNT,
        REQUIRED_COLLATERAL,
        RENT_PER_SECOND,
        DURATION_SECONDS,
        MAINTENANCE_MARGIN_BPS,
        LIQUIDATION_BOUNTY_BPS,
        FEED_ID,
    );
    let take_ix = build_take_lease_ix(&sc, lease_id);
    send_transaction_from_instructions(
        &mut sc.svm,
        vec![create_ix, take_ix],
        &[&sc.lessor, &sc.lessee],
        &sc.lessor.pubkey(),
    )
    .unwrap();

    // A bit of rent accrues before the liquidation call so the handler has to
    // settle rent *and* bounty on the same vault balance.
    let elapsed: i64 = 300;
    advance_clock_by(&mut sc.svm, elapsed);

    // Install a Pyth price that quotes leased-in-collateral at 4.0 per unit
    // with exponent 0. At 100 leased units the debt is 400 collateral units
    // vs. the 200 collateral we hold — ratio 50%, well below 120% margin.
    let price_update_key = Keypair::new();
    let now = current_clock(&sc.svm);
    mock_price_update(
        &mut sc.svm,
        price_update_key.pubkey(),
        FEED_ID,
        4,
        0,
        now, // fresh publish_time
    );

    let liq_ix = build_liquidate_ix(&sc, lease_id, price_update_key.pubkey());
    send_transaction_from_instructions(
        &mut sc.svm,
        vec![liq_ix],
        &[&sc.keeper],
        &sc.keeper.pubkey(),
    )
    .unwrap();

    let rent_paid = (elapsed as u64) * RENT_PER_SECOND;
    let remaining_after_rent = REQUIRED_COLLATERAL - rent_paid;
    let bounty = remaining_after_rent * (LIQUIDATION_BOUNTY_BPS as u64) / 10_000;
    let lessor_share = remaining_after_rent - bounty;

    let lessor_collateral_ata = derive_ata(&sc.lessor.pubkey(), &sc.collateral_mint);
    let keeper_collateral_ata = derive_ata(&sc.keeper.pubkey(), &sc.collateral_mint);

    assert_eq!(
        get_token_account_balance(&sc.svm, &lessor_collateral_ata).unwrap(),
        rent_paid + lessor_share
    );
    assert_eq!(
        get_token_account_balance(&sc.svm, &keeper_collateral_ata).unwrap(),
        bounty
    );

    // Vaults and lease account closed.
    let (lease_pda, leased_vault, collateral_vault) =
        lease_pdas(&sc.program_id, &sc.lessor.pubkey(), lease_id);
    assert!(sc.svm.get_account(&lease_pda).is_none());
    assert!(sc.svm.get_account(&leased_vault).is_none());
    assert!(sc.svm.get_account(&collateral_vault).is_none());
}

#[test]
fn liquidate_rejects_healthy_position() {
    let mut sc = full_setup();
    let lease_id = 7u64;

    let create_ix = build_create_lease_ix(
        &sc,
        lease_id,
        LEASED_AMOUNT,
        REQUIRED_COLLATERAL,
        RENT_PER_SECOND,
        DURATION_SECONDS,
        MAINTENANCE_MARGIN_BPS,
        LIQUIDATION_BOUNTY_BPS,
        FEED_ID,
    );
    let take_ix = build_take_lease_ix(&sc, lease_id);
    send_transaction_from_instructions(
        &mut sc.svm,
        vec![create_ix, take_ix],
        &[&sc.lessor, &sc.lessee],
        &sc.lessor.pubkey(),
    )
    .unwrap();

    // Price of 1.0 per leased token → debt = 100 collateral units, collateral
    // = 200 → ratio 200% ≥ 120% maintenance margin. Expect the transaction
    // to fail with `PositionHealthy`.
    let price_update_key = Keypair::new();
    let now = current_clock(&sc.svm);
    mock_price_update(&mut sc.svm, price_update_key.pubkey(), FEED_ID, 1, 0, now);

    let liq_ix = build_liquidate_ix(&sc, lease_id, price_update_key.pubkey());
    let result = send_transaction_from_instructions(
        &mut sc.svm,
        vec![liq_ix],
        &[&sc.keeper],
        &sc.keeper.pubkey(),
    );
    assert!(result.is_err(), "healthy liquidation must fail");
}

#[test]
fn liquidate_rejects_mismatched_price_feed() {
    // The lessor pinned FEED_ID; we hand the handler a price update whose
    // internal feed_id is different. Even when the price would push the
    // position underwater, the liquidate call must bail with
    // `PriceFeedMismatch` before running the undercollateralisation check.
    let mut sc = full_setup();
    let lease_id = 100u64;

    let create_ix = build_create_lease_ix(
        &sc,
        lease_id,
        LEASED_AMOUNT,
        REQUIRED_COLLATERAL,
        RENT_PER_SECOND,
        DURATION_SECONDS,
        MAINTENANCE_MARGIN_BPS,
        LIQUIDATION_BOUNTY_BPS,
        FEED_ID,
    );
    let take_ix = build_take_lease_ix(&sc, lease_id);
    send_transaction_from_instructions(
        &mut sc.svm,
        vec![create_ix, take_ix],
        &[&sc.lessor, &sc.lessee],
        &sc.lessor.pubkey(),
    )
    .unwrap();

    // Flip every byte — any 32-byte feed id other than FEED_ID should do.
    let wrong_feed_id = [0xCD; 32];

    // Price that *would* trigger liquidation (debt 400 vs 200 collateral,
    // same as `liquidate_seizes_collateral_on_price_drop`) — except this
    // update carries the wrong feed id.
    let price_update_key = Keypair::new();
    let now = current_clock(&sc.svm);
    mock_price_update(
        &mut sc.svm,
        price_update_key.pubkey(),
        wrong_feed_id,
        4,
        0,
        now,
    );

    let liq_ix = build_liquidate_ix(&sc, lease_id, price_update_key.pubkey());
    let result = send_transaction_from_instructions(
        &mut sc.svm,
        vec![liq_ix],
        &[&sc.keeper],
        &sc.keeper.pubkey(),
    );
    let err = result.expect_err("liquidate must reject foreign price feeds");
    let rendered = format!("{err:?}");
    // PriceFeedMismatch is the 16th error in the enum (index 15) → 0x177f.
    assert!(
        rendered.contains("PriceFeedMismatch") || rendered.contains("0x177f"),
        "unexpected failure mode: {rendered}"
    );
}

#[test]
fn close_expired_reclaims_collateral_after_end_ts() {
    let mut sc = full_setup();
    let lease_id = 8u64;

    let create_ix = build_create_lease_ix(
        &sc,
        lease_id,
        LEASED_AMOUNT,
        REQUIRED_COLLATERAL,
        RENT_PER_SECOND,
        DURATION_SECONDS,
        MAINTENANCE_MARGIN_BPS,
        LIQUIDATION_BOUNTY_BPS,
        FEED_ID,
    );
    let take_ix = build_take_lease_ix(&sc, lease_id);
    send_transaction_from_instructions(
        &mut sc.svm,
        vec![create_ix, take_ix],
        &[&sc.lessor, &sc.lessee],
        &sc.lessor.pubkey(),
    )
    .unwrap();

    // Jump past the lease end.
    advance_clock_by(&mut sc.svm, DURATION_SECONDS + 1);

    let close_ix = build_close_expired_ix(&sc, lease_id);
    send_transaction_from_instructions(
        &mut sc.svm,
        vec![close_ix],
        &[&sc.lessor],
        &sc.lessor.pubkey(),
    )
    .unwrap();

    // Full collateral forfeited to the lessor. Leased tokens are gone (the
    // lessee kept them on default) so the lessor's leased balance is only
    // what they had after the initial escrow minus the leased amount.
    let lessor_collateral_ata = derive_ata(&sc.lessor.pubkey(), &sc.collateral_mint);
    assert_eq!(
        get_token_account_balance(&sc.svm, &lessor_collateral_ata).unwrap(),
        REQUIRED_COLLATERAL
    );
    assert_eq!(
        get_token_account_balance(&sc.svm, &sc.lessor_leased_ata).unwrap(),
        1_000_000_000 - LEASED_AMOUNT
    );

    let (lease_pda, leased_vault, collateral_vault) =
        lease_pdas(&sc.program_id, &sc.lessor.pubkey(), lease_id);
    assert!(sc.svm.get_account(&lease_pda).is_none());
    assert!(sc.svm.get_account(&leased_vault).is_none());
    assert!(sc.svm.get_account(&collateral_vault).is_none());
}

#[test]
fn close_expired_cancels_listed_lease() {
    let mut sc = full_setup();
    let lease_id = 9u64;

    let create_ix = build_create_lease_ix(
        &sc,
        lease_id,
        LEASED_AMOUNT,
        REQUIRED_COLLATERAL,
        RENT_PER_SECOND,
        DURATION_SECONDS,
        MAINTENANCE_MARGIN_BPS,
        LIQUIDATION_BOUNTY_BPS,
        FEED_ID,
    );
    send_transaction_from_instructions(
        &mut sc.svm,
        vec![create_ix],
        &[&sc.lessor],
        &sc.lessor.pubkey(),
    )
    .unwrap();

    // Lessor bails before anyone takes the lease — allowed immediately.
    let close_ix = build_close_expired_ix(&sc, lease_id);
    send_transaction_from_instructions(
        &mut sc.svm,
        vec![close_ix],
        &[&sc.lessor],
        &sc.lessor.pubkey(),
    )
    .unwrap();

    // Lessor recovered the full leased amount. No collateral was ever posted.
    assert_eq!(
        get_token_account_balance(&sc.svm, &sc.lessor_leased_ata).unwrap(),
        1_000_000_000
    );
    let (lease_pda, leased_vault, collateral_vault) =
        lease_pdas(&sc.program_id, &sc.lessor.pubkey(), lease_id);
    assert!(sc.svm.get_account(&lease_pda).is_none());
    assert!(sc.svm.get_account(&leased_vault).is_none());
    assert!(sc.svm.get_account(&collateral_vault).is_none());
}

#[test]
fn create_lease_rejects_same_mint_for_leased_and_collateral() {
    // Collapsing leased_mint and collateral_mint into a single SPL mint would
    // also collapse the two vaults into one token-balance pool (same mint,
    // same authority seed pattern) and make rent-vs-collateral accounting
    // ambiguous. The program rejects this up-front with
    // `LeasedMintEqualsCollateralMint`.
    let mut sc = full_setup();
    let lease_id = 42u64;

    // Build a `create_lease` instruction where the collateral_mint field
    // carries the same mint as leased_mint. We bypass `build_create_lease_ix`
    // because that helper always wires the two mints from the scenario.
    let (lease, leased_vault, collateral_vault) =
        lease_pdas(&sc.program_id, &sc.lessor.pubkey(), lease_id);
    let ix = Instruction::new_with_bytes(
        sc.program_id,
        &asset_leasing::instruction::CreateLease {
            lease_id,
            leased_amount: LEASED_AMOUNT,
            required_collateral_amount: REQUIRED_COLLATERAL,
            rent_per_second: RENT_PER_SECOND,
            duration_seconds: DURATION_SECONDS,
            maintenance_margin_bps: MAINTENANCE_MARGIN_BPS,
            liquidation_bounty_bps: LIQUIDATION_BOUNTY_BPS,
            feed_id: FEED_ID,
        }
        .data(),
        asset_leasing::accounts::CreateLease {
            lessor: sc.lessor.pubkey(),
            leased_mint: sc.leased_mint,
            // Same mint on both sides — should be rejected.
            collateral_mint: sc.leased_mint,
            lessor_leased_account: sc.lessor_leased_ata,
            lease,
            leased_vault,
            collateral_vault,
            token_program: token_program_id(),
            system_program: system_program::id(),
        }
        .to_account_metas(None),
    );

    let result = send_transaction_from_instructions(
        &mut sc.svm,
        vec![ix],
        &[&sc.lessor],
        &sc.lessor.pubkey(),
    );

    let err = result.expect_err("create_lease must reject identical leased/collateral mints");
    let rendered = format!("{err:?}");
    assert!(
        rendered.contains("LeasedMintEqualsCollateralMint") || rendered.contains("0x177e"),
        "unexpected failure mode: {rendered}"
    );
}
