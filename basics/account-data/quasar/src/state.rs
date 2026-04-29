use quasar_lang::prelude::*;

/// Onchain address info account with dynamic string fields.
/// Note: Quasar requires all fixed-size fields to precede dynamic (String/Vec) fields.
#[account(discriminator = 1, set_inner)]
#[seeds(b"address_info", payer: Address)]
pub struct AddressInfo {
    pub house_number: u8,
    pub name: String<50>,
    pub street: String<50>,
    pub city: String<50>,
}
