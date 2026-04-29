use quasar_svm::{Account, Instruction, Pubkey, QuasarSvm};
use solana_address::Address;

fn setup() -> QuasarSvm {
    let elf = include_bytes!("../target/deploy/quasar_account_data.so");
    QuasarSvm::new().with_program(&Pubkey::from(crate::ID), elf)
}

fn signer(address: Pubkey) -> Account {
    quasar_svm::token::create_keyed_system_account(&address, 10_000_000_000)
}

fn empty(address: Pubkey) -> Account {
    Account {
        address,
        lamports: 0,
        data: vec![],
        owner: quasar_svm::system_program::ID,
        executable: false,
    }
}

/// Build the create_address_info instruction data using Quasar's compact
/// wire format: a header containing all fixed fields and length prefixes,
/// followed by a tail with all dynamic byte payloads grouped together.
///
/// Layout:
///   header: [disc: u8 = 0][house_number: u8][name_len: u8][street_len: u8][city_len: u8]
///   tail:   [name bytes][street bytes][city bytes]
///
/// `String<50>` defaults to a u8 length prefix because MAX (50) fits in a byte.
fn build_create_instruction_data(name: &str, house_number: u8, street: &str, city: &str) -> Vec<u8> {
    let mut data = Vec::with_capacity(5 + name.len() + street.len() + city.len());

    // Header
    data.push(0u8); // instruction discriminator
    data.push(house_number);
    data.push(name.len() as u8);
    data.push(street.len() as u8);
    data.push(city.len() as u8);

    // Tail
    data.extend_from_slice(name.as_bytes());
    data.extend_from_slice(street.as_bytes());
    data.extend_from_slice(city.as_bytes());

    data
}

#[test]
fn test_create_address_info() {
    let mut svm = setup();

    let payer = Pubkey::new_unique();
    let system_program = quasar_svm::system_program::ID;

    let (address_info, _) = Pubkey::find_program_address(
        &[b"address_info", payer.as_ref()],
        &Pubkey::from(crate::ID),
    );

    let data = build_create_instruction_data("Alice", 42, "Main Street", "New York");

    let instruction = Instruction {
        program_id: Pubkey::from(crate::ID),
        accounts: vec![
            solana_instruction::AccountMeta::new(Address::from(payer.to_bytes()), true),
            solana_instruction::AccountMeta::new(Address::from(address_info.to_bytes()), false),
            solana_instruction::AccountMeta::new_readonly(
                Address::from(system_program.to_bytes()),
                false,
            ),
        ],
        data,
    };

    let result = svm.process_instruction(&instruction, &[signer(payer), empty(address_info)]);

    result.assert_success();

    // Verify the account data.
    let account = result.account(&address_info).unwrap();

    // Onchain layout for a Quasar `#[account]` with dynamic fields uses the
    // compact "header then tail" format. Length prefixes are grouped in the
    // header, the actual bytes follow in the tail.
    //   header: [disc: 1][house_number: u8][name_len: u8][street_len: u8][city_len: u8]
    //   tail:   [name bytes][street bytes][city bytes]
    // String<50> defaults to a u8 length prefix because MAX (50) fits in a byte.
    assert_eq!(account.data[0], 1, "discriminator");
    assert_eq!(account.data[1], 42, "house_number");
    let name_len = account.data[2] as usize;
    let street_len = account.data[3] as usize;
    let city_len = account.data[4] as usize;
    assert_eq!(name_len, 5);
    assert_eq!(street_len, 11);
    assert_eq!(city_len, 8);

    let header_end = 5;
    assert_eq!(&account.data[header_end..header_end + name_len], b"Alice");
    let street_start = header_end + name_len;
    assert_eq!(&account.data[street_start..street_start + street_len], b"Main Street");
    let city_start = street_start + street_len;
    assert_eq!(&account.data[city_start..city_start + city_len], b"New York");
}
