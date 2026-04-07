use anchor_lang::prelude::*;
use anchor_spl::token_interface::{
    spl_pod::optional_keys::OptionalNonZeroPubkey,
    spl_token_2022::{
        extension::{
            interest_bearing_mint::InterestBearingConfig, BaseStateWithExtensions,
            StateWithExtensions,
        },
        state::Mint as MintState,
    },
};

pub fn check_mint_data(mint_account_info: &AccountInfo, authority_key: &Pubkey) -> Result<()> {
    let mint_data = mint_account_info.data.borrow();
    let mint_with_extension = StateWithExtensions::<MintState>::unpack(&mint_data)?;
    let extension_data = mint_with_extension.get_extension::<InterestBearingConfig>()?;

    assert_eq!(
        extension_data.rate_authority,
        OptionalNonZeroPubkey::try_from(Some(*authority_key))?
    );

    msg!("{:?}", extension_data);
    Ok(())
}
