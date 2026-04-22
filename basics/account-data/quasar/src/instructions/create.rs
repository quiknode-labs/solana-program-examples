use {
    crate::state::AddressInfo,
    quasar_lang::prelude::*,
};

/// Accounts for creating a new address info account.
/// Dynamic accounts use owned `Account<T>` rather than `&'info mut Account<T>` because
/// dynamic types carry cached byte offsets that cannot be represented as a pointer cast.
#[derive(Accounts)]
pub struct CreateAddressInfo {
    #[account(mut)]
    pub payer: Signer,
    #[account(mut, init, payer = payer, seeds = AddressInfo::seeds(payer), bump)]
    pub address_info: Account<AddressInfo<'_>>,
    pub system_program: Program<System>,
}

impl CreateAddressInfo {
    #[inline(always)]
    pub fn create_address_info(
        &mut self, name: &str,
        house_number: u8,
        street: &str,
        city: &str,
    ) -> Result<(), ProgramError> {
        self.address_info.set_inner(
            house_number,
            name,
            street,
            city,
            self.payer.to_account_view(),
            None,
        )
    }
}
