use {
    anchor_lang::{InstructionData, ToAccountMetas},
    asset_leasing::{
        accounts::{
            CollectFeesAccountConstraints, DelistAssetAccountConstraints,
            InitializeAccountConstraints, ListAssetAccountConstraints,
            RentAssetAccountConstraints, ReturnAssetAccountConstraints,
        },
        instruction,
    },
    litesvm::LiteSVM,
    solana_instruction::Instruction,
    solana_kite::{
        create_associated_token_account, create_token_mint, create_wallet,
        get_token_account_address, mint_tokens_to_token_account, send_transaction_from_instructions,
    },
    solana_pubkey::Pubkey,
    solana_signer::Signer,
    std::path::PathBuf,
};

const PROGRAM_ID: Pubkey = asset_leasing::ID_CONST;

fn deploy_program() -> LiteSVM {
    let mut svm = LiteSVM::new();

    let mut so_path = PathBuf::from(env!("CARGO_MANIFEST_DIR"));
    so_path.push("../../target/deploy/asset_leasing.so");

    let bytecode = std::fs::read(&so_path)
        .unwrap_or_else(|_| panic!("Run `anchor build` first. Missing: {}", so_path.display()));

    svm.add_program(PROGRAM_ID, &bytecode)
        .expect("Failed to deploy program");
    svm
}

fn find_pda(seeds: &[&[u8]]) -> (Pubkey, u8) {
    Pubkey::find_program_address(seeds, &PROGRAM_ID)
}

fn lease_config_pda() -> (Pubkey, u8) {
    find_pda(&[b"lease_config"])
}

fn listing_pda(owner: &Pubkey, mint: &Pubkey) -> (Pubkey, u8) {
    find_pda(&[b"listing", owner.as_ref(), mint.as_ref()])
}

fn lease_pda(listing: &Pubkey, renter: &Pubkey) -> (Pubkey, u8) {
    find_pda(&[b"lease", listing.as_ref(), renter.as_ref()])
}

// ─── Initialize ─────────────────────────────────────────────

#[test]
fn test_initialize() {
    let mut svm = deploy_program();
    let authority = create_wallet(&mut svm, 10_000_000_000).unwrap();
    let (config_pda, _) = lease_config_pda();

    let accounts = InitializeAccountConstraints {
        authority: authority.pubkey(),
        lease_config: config_pda,
        system_program: solana_pubkey::pubkey!("11111111111111111111111111111111"),
    };

    let ix = Instruction {
        program_id: PROGRAM_ID,
        accounts: accounts.to_account_metas(None),
        data: instruction::Initialize {
            fee_basis_points: 250,
        }
        .data(),
    };

    send_transaction_from_instructions(&mut svm, vec![ix], &[&authority], &authority.pubkey())
        .unwrap();
}

// ─── Full lifecycle: list → rent → return → delist ──────────

#[test]
fn test_full_lease_lifecycle() {
    let mut svm = deploy_program();

    let authority = create_wallet(&mut svm, 10_000_000_000).unwrap();
    let owner = create_wallet(&mut svm, 10_000_000_000).unwrap();
    let renter = create_wallet(&mut svm, 10_000_000_000).unwrap();

    // --- Initialize program ---
    let (config_pda, _) = lease_config_pda();
    let system_program = solana_pubkey::pubkey!("11111111111111111111111111111111");

    let init_ix = Instruction {
        program_id: PROGRAM_ID,
        accounts: InitializeAccountConstraints {
            authority: authority.pubkey(),
            lease_config: config_pda,
            system_program,
        }
        .to_account_metas(None),
        data: instruction::Initialize {
            fee_basis_points: 250,
        }
        .data(),
    };
    send_transaction_from_instructions(
        &mut svm,
        vec![init_ix],
        &[&authority],
        &authority.pubkey(),
    )
    .unwrap();

    // --- Create a token mint and give owner some tokens ---
    let mint_pubkey =
        create_token_mint(&mut svm, &owner, 0, None).unwrap();

    let owner_ata =
        create_associated_token_account(&mut svm, &owner.pubkey(), &mint_pubkey, &owner).unwrap();

    // Mint 1 NFT-like token to the owner
    mint_tokens_to_token_account(&mut svm, &mint_pubkey, &owner_ata, 1, &owner).unwrap();

    // --- List the asset ---
    let (listing_pda_key, _) = listing_pda(&owner.pubkey(), &mint_pubkey);
    let vault_ata = get_token_account_address(&listing_pda_key, &mint_pubkey);

    let ata_program = solana_pubkey::pubkey!("ATokenGPvbdGVxr1b2hvZbsiqW5xWH25efTNsLJA8knL");
    let token_program = solana_pubkey::pubkey!("TokenkegQfeZyiNwAJbNbGKPFXCWuBvf9Ss623VQ5DA");

    let list_ix = Instruction {
        program_id: PROGRAM_ID,
        accounts: ListAssetAccountConstraints {
            owner: owner.pubkey(),
            asset_mint: mint_pubkey,
            owner_token_account: owner_ata,
            listing: listing_pda_key,
            vault: vault_ata,
            associated_token_program: ata_program,
            token_program,
            system_program,
        }
        .to_account_metas(None),
        data: instruction::ListAsset {
            price_per_second: 1_000, // 1000 lamports/second
            min_duration: 10,        // 10 seconds minimum
            max_duration: 3600,      // 1 hour maximum
            amount: 1,
        }
        .data(),
    };
    send_transaction_from_instructions(&mut svm, vec![list_ix], &[&owner], &owner.pubkey())
        .unwrap();

    // Verify the token moved to vault
    let owner_balance =
        solana_kite::get_token_account_balance(&svm, &owner_ata).unwrap();
    assert_eq!(owner_balance, 0, "Owner should have 0 tokens after listing");

    let vault_balance =
        solana_kite::get_token_account_balance(&svm, &vault_ata).unwrap();
    assert_eq!(vault_balance, 1, "Vault should have 1 token");

    // --- Rent the asset ---
    let renter_ata = get_token_account_address(&renter.pubkey(), &mint_pubkey);
    let (lease_pda_key, _) = lease_pda(&listing_pda_key, &renter.pubkey());

    let owner_sol_before = svm.get_balance(&owner.pubkey()).unwrap();

    let rent_ix = Instruction {
        program_id: PROGRAM_ID,
        accounts: RentAssetAccountConstraints {
            renter: renter.pubkey(),
            owner: owner.pubkey(),
            asset_mint: mint_pubkey,
            lease_config: config_pda,
            fee_collector: authority.pubkey(),
            listing: listing_pda_key,
            vault: vault_ata,
            renter_token_account: renter_ata,
            lease: lease_pda_key,
            associated_token_program: ata_program,
            token_program,
            system_program,
        }
        .to_account_metas(None),
        data: instruction::RentAsset { duration: 60 }.data(), // 60 seconds
    };
    send_transaction_from_instructions(&mut svm, vec![rent_ix], &[&renter], &renter.pubkey())
        .unwrap();

    // Verify token moved to renter
    let renter_balance =
        solana_kite::get_token_account_balance(&svm, &renter_ata).unwrap();
    assert_eq!(renter_balance, 1, "Renter should have 1 token after renting");

    // Verify owner received SOL payment (60 seconds * 1000 lamports = 60,000 total)
    // Owner gets 97.5% (after 2.5% fee) = 58,500 lamports
    let owner_sol_after = svm.get_balance(&owner.pubkey()).unwrap();
    let owner_received = owner_sol_after - owner_sol_before;
    assert_eq!(
        owner_received, 58_500,
        "Owner should receive 97.5% of rental payment"
    );

    // --- Return the asset ---
    let return_ix = Instruction {
        program_id: PROGRAM_ID,
        accounts: ReturnAssetAccountConstraints {
            renter: renter.pubkey(),
            asset_mint: mint_pubkey,
            owner: owner.pubkey(),
            listing: listing_pda_key,
            vault: vault_ata,
            renter_token_account: renter_ata,
            lease: lease_pda_key,
            token_program,
        }
        .to_account_metas(None),
        data: instruction::ReturnAsset {}.data(),
    };
    send_transaction_from_instructions(&mut svm, vec![return_ix], &[&renter], &renter.pubkey())
        .unwrap();

    // Verify token back in vault
    let vault_balance_after =
        solana_kite::get_token_account_balance(&svm, &vault_ata).unwrap();
    assert_eq!(vault_balance_after, 1, "Vault should have token back");

    let renter_balance_after =
        solana_kite::get_token_account_balance(&svm, &renter_ata).unwrap();
    assert_eq!(renter_balance_after, 0, "Renter should have 0 tokens");

    // --- Delist the asset ---
    let delist_ix = Instruction {
        program_id: PROGRAM_ID,
        accounts: DelistAssetAccountConstraints {
            owner: owner.pubkey(),
            asset_mint: mint_pubkey,
            owner_token_account: owner_ata,
            listing: listing_pda_key,
            vault: vault_ata,
            associated_token_program: ata_program,
            token_program,
            system_program,
        }
        .to_account_metas(None),
        data: instruction::DelistAsset {}.data(),
    };
    send_transaction_from_instructions(&mut svm, vec![delist_ix], &[&owner], &owner.pubkey())
        .unwrap();

    // Verify token back with owner
    let owner_balance_final =
        solana_kite::get_token_account_balance(&svm, &owner_ata).unwrap();
    assert_eq!(
        owner_balance_final, 1,
        "Owner should have token back after delist"
    );
}

// ─── Cannot delist while leased ─────────────────────────────

#[test]
fn test_cannot_delist_while_leased() {
    let mut svm = deploy_program();

    let authority = create_wallet(&mut svm, 10_000_000_000).unwrap();
    let owner = create_wallet(&mut svm, 10_000_000_000).unwrap();
    let renter = create_wallet(&mut svm, 10_000_000_000).unwrap();

    let system_program = solana_pubkey::pubkey!("11111111111111111111111111111111");
    let ata_program = solana_pubkey::pubkey!("ATokenGPvbdGVxr1b2hvZbsiqW5xWH25efTNsLJA8knL");
    let token_program = solana_pubkey::pubkey!("TokenkegQfeZyiNwAJbNbGKPFXCWuBvf9Ss623VQ5DA");

    // Initialize
    let (config_pda, _) = lease_config_pda();
    let init_ix = Instruction {
        program_id: PROGRAM_ID,
        accounts: InitializeAccountConstraints {
            authority: authority.pubkey(),
            lease_config: config_pda,
            system_program,
        }
        .to_account_metas(None),
        data: instruction::Initialize {
            fee_basis_points: 250,
        }
        .data(),
    };
    send_transaction_from_instructions(
        &mut svm,
        vec![init_ix],
        &[&authority],
        &authority.pubkey(),
    )
    .unwrap();

    // Create mint + list
    let mint = create_token_mint(&mut svm, &owner, 0, None).unwrap();
    let owner_ata = create_associated_token_account(&mut svm, &owner.pubkey(), &mint, &owner).unwrap();
    mint_tokens_to_token_account(&mut svm, &mint, &owner_ata, 1, &owner).unwrap();

    let (listing_key, _) = listing_pda(&owner.pubkey(), &mint);
    let vault_ata = get_token_account_address(&listing_key, &mint);

    let list_ix = Instruction {
        program_id: PROGRAM_ID,
        accounts: ListAssetAccountConstraints {
            owner: owner.pubkey(),
            asset_mint: mint,
            owner_token_account: owner_ata,
            listing: listing_key,
            vault: vault_ata,
            associated_token_program: ata_program,
            token_program,
            system_program,
        }
        .to_account_metas(None),
        data: instruction::ListAsset {
            price_per_second: 1_000,
            min_duration: 10,
            max_duration: 3600,
            amount: 1,
        }
        .data(),
    };
    send_transaction_from_instructions(&mut svm, vec![list_ix], &[&owner], &owner.pubkey())
        .unwrap();

    // Rent
    let renter_ata = get_token_account_address(&renter.pubkey(), &mint);
    let (lease_key, _) = lease_pda(&listing_key, &renter.pubkey());

    let rent_ix = Instruction {
        program_id: PROGRAM_ID,
        accounts: RentAssetAccountConstraints {
            renter: renter.pubkey(),
            owner: owner.pubkey(),
            asset_mint: mint,
            lease_config: config_pda,
            fee_collector: authority.pubkey(),
            listing: listing_key,
            vault: vault_ata,
            renter_token_account: renter_ata,
            lease: lease_key,
            associated_token_program: ata_program,
            token_program,
            system_program,
        }
        .to_account_metas(None),
        data: instruction::RentAsset { duration: 60 }.data(),
    };
    send_transaction_from_instructions(&mut svm, vec![rent_ix], &[&renter], &renter.pubkey())
        .unwrap();

    // Try to delist — should fail because there's an active lease
    let delist_ix = Instruction {
        program_id: PROGRAM_ID,
        accounts: DelistAssetAccountConstraints {
            owner: owner.pubkey(),
            asset_mint: mint,
            owner_token_account: owner_ata,
            listing: listing_key,
            vault: vault_ata,
            associated_token_program: ata_program,
            token_program,
            system_program,
        }
        .to_account_metas(None),
        data: instruction::DelistAsset {}.data(),
    };

    let result =
        send_transaction_from_instructions(&mut svm, vec![delist_ix], &[&owner], &owner.pubkey());
    assert!(result.is_err(), "Should not be able to delist while leased");
}

// ─── Collect fees (update fee rate) ─────────────────────────

#[test]
fn test_collect_fees_updates_rate() {
    let mut svm = deploy_program();
    let authority = create_wallet(&mut svm, 10_000_000_000).unwrap();
    let (config_pda, _) = lease_config_pda();
    let system_program = solana_pubkey::pubkey!("11111111111111111111111111111111");

    // Initialize with 2.5%
    let init_ix = Instruction {
        program_id: PROGRAM_ID,
        accounts: InitializeAccountConstraints {
            authority: authority.pubkey(),
            lease_config: config_pda,
            system_program,
        }
        .to_account_metas(None),
        data: instruction::Initialize {
            fee_basis_points: 250,
        }
        .data(),
    };
    send_transaction_from_instructions(
        &mut svm,
        vec![init_ix],
        &[&authority],
        &authority.pubkey(),
    )
    .unwrap();

    // Update to 5%
    let collect_ix = Instruction {
        program_id: PROGRAM_ID,
        accounts: CollectFeesAccountConstraints {
            authority: authority.pubkey(),
            lease_config: config_pda,
        }
        .to_account_metas(None),
        data: instruction::CollectFees {
            new_fee_basis_points: 500,
        }
        .data(),
    };
    send_transaction_from_instructions(
        &mut svm,
        vec![collect_ix],
        &[&authority],
        &authority.pubkey(),
    )
    .unwrap();

    // Try setting fee too high (> 100%) — should fail
    let bad_collect_ix = Instruction {
        program_id: PROGRAM_ID,
        accounts: CollectFeesAccountConstraints {
            authority: authority.pubkey(),
            lease_config: config_pda,
        }
        .to_account_metas(None),
        data: instruction::CollectFees {
            new_fee_basis_points: 10_001,
        }
        .data(),
    };
    let result = send_transaction_from_instructions(
        &mut svm,
        vec![bad_collect_ix],
        &[&authority],
        &authority.pubkey(),
    );
    assert!(result.is_err(), "Fee > 100% should be rejected");
}

// ─── Duration validation ────────────────────────────────────

#[test]
fn test_duration_validation() {
    let mut svm = deploy_program();

    let authority = create_wallet(&mut svm, 10_000_000_000).unwrap();
    let owner = create_wallet(&mut svm, 10_000_000_000).unwrap();
    let renter = create_wallet(&mut svm, 10_000_000_000).unwrap();

    let system_program = solana_pubkey::pubkey!("11111111111111111111111111111111");
    let ata_program = solana_pubkey::pubkey!("ATokenGPvbdGVxr1b2hvZbsiqW5xWH25efTNsLJA8knL");
    let token_program = solana_pubkey::pubkey!("TokenkegQfeZyiNwAJbNbGKPFXCWuBvf9Ss623VQ5DA");

    // Initialize
    let (config_pda, _) = lease_config_pda();
    let init_ix = Instruction {
        program_id: PROGRAM_ID,
        accounts: InitializeAccountConstraints {
            authority: authority.pubkey(),
            lease_config: config_pda,
            system_program,
        }
        .to_account_metas(None),
        data: instruction::Initialize {
            fee_basis_points: 250,
        }
        .data(),
    };
    send_transaction_from_instructions(
        &mut svm,
        vec![init_ix],
        &[&authority],
        &authority.pubkey(),
    )
    .unwrap();

    // Create mint + list with min=60, max=3600
    let mint = create_token_mint(&mut svm, &owner, 0, None).unwrap();
    let owner_ata = create_associated_token_account(&mut svm, &owner.pubkey(), &mint, &owner).unwrap();
    mint_tokens_to_token_account(&mut svm, &mint, &owner_ata, 1, &owner).unwrap();

    let (listing_key, _) = listing_pda(&owner.pubkey(), &mint);
    let vault_ata = get_token_account_address(&listing_key, &mint);

    let list_ix = Instruction {
        program_id: PROGRAM_ID,
        accounts: ListAssetAccountConstraints {
            owner: owner.pubkey(),
            asset_mint: mint,
            owner_token_account: owner_ata,
            listing: listing_key,
            vault: vault_ata,
            associated_token_program: ata_program,
            token_program,
            system_program,
        }
        .to_account_metas(None),
        data: instruction::ListAsset {
            price_per_second: 1_000,
            min_duration: 60,
            max_duration: 3600,
            amount: 1,
        }
        .data(),
    };
    send_transaction_from_instructions(&mut svm, vec![list_ix], &[&owner], &owner.pubkey())
        .unwrap();

    // Try renting for 5 seconds (below min of 60) — should fail
    let renter_ata = get_token_account_address(&renter.pubkey(), &mint);
    let (lease_key, _) = lease_pda(&listing_key, &renter.pubkey());

    let too_short_ix = Instruction {
        program_id: PROGRAM_ID,
        accounts: RentAssetAccountConstraints {
            renter: renter.pubkey(),
            owner: owner.pubkey(),
            asset_mint: mint,
            lease_config: config_pda,
            fee_collector: authority.pubkey(),
            listing: listing_key,
            vault: vault_ata,
            renter_token_account: renter_ata,
            lease: lease_key,
            associated_token_program: ata_program,
            token_program,
            system_program,
        }
        .to_account_metas(None),
        data: instruction::RentAsset { duration: 5 }.data(),
    };

    let result = send_transaction_from_instructions(
        &mut svm,
        vec![too_short_ix],
        &[&renter],
        &renter.pubkey(),
    );
    assert!(
        result.is_err(),
        "Duration below minimum should be rejected"
    );

    // Try renting for 7200 seconds (above max of 3600) — should fail
    let too_long_ix = Instruction {
        program_id: PROGRAM_ID,
        accounts: RentAssetAccountConstraints {
            renter: renter.pubkey(),
            owner: owner.pubkey(),
            asset_mint: mint,
            lease_config: config_pda,
            fee_collector: authority.pubkey(),
            listing: listing_key,
            vault: vault_ata,
            renter_token_account: renter_ata,
            lease: lease_key,
            associated_token_program: ata_program,
            token_program,
            system_program,
        }
        .to_account_metas(None),
        data: instruction::RentAsset { duration: 7200 }.data(),
    };

    let result = send_transaction_from_instructions(
        &mut svm,
        vec![too_long_ix],
        &[&renter],
        &renter.pubkey(),
    );
    assert!(
        result.is_err(),
        "Duration above maximum should be rejected"
    );
}
