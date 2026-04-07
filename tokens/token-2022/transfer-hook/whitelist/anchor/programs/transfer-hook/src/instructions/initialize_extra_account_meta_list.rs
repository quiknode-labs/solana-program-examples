use anchor_lang::prelude::*;
use anchor_spl::token_interface::Mint;
use spl_tlv_account_resolution::{
    account::ExtraAccountMeta, seeds::Seed, state::ExtraAccountMetaList,
};
use spl_discriminator::SplDiscriminate;
use spl_transfer_hook_interface::instruction::ExecuteInstruction;

use crate::state::WhiteList;

#[derive(Accounts)]
pub struct InitializeExtraAccountMetaList<'info> {
    #[account(mut)]
    payer: Signer<'info>,

    /// CHECK: ExtraAccountMetaList Account, must use these seeds
    #[account(
        init,
        seeds = [b"extra-account-metas", mint.key().as_ref()],
        bump,
        // size_of returns Result with spl's ProgramError — unwrap is safe for known-good input
        space = ExtraAccountMetaList::size_of(
            InitializeExtraAccountMetaList::extra_account_metas_count()
        ).unwrap(),
        payer = payer
    )]
    pub extra_account_meta_list: UncheckedAccount<'info>,
    pub mint: InterfaceAccount<'info, Mint>,
    pub system_program: Program<'info, System>,
    #[account(init_if_needed, seeds = [b"white_list"], bump, payer = payer, space = 400)]
    pub white_list: Account<'info, WhiteList>,
}

// Define extra account metas to store on extra_account_meta_list account
impl<'info> InitializeExtraAccountMetaList<'info> {
    pub fn extra_account_metas() -> Result<Vec<ExtraAccountMeta>> {
        // .map_err() needed because spl-tlv-account-resolution uses solana-program-error 2.x
        // while anchor-lang 1.0 uses 3.x — structurally identical but different semver types
        Ok(vec![ExtraAccountMeta::new_with_seeds(
            &[Seed::Literal {
                bytes: "white_list".as_bytes().to_vec(),
            }],
            false, // is_signer
            true,  // is_writable
        ).map_err(|_| ProgramError::InvalidArgument)?])
    }

    /// Returns the count of extra account metas (avoids the error conversion issue in #[account] attributes)
    pub fn extra_account_metas_count() -> usize {
        1 // one extra account: the whitelist PDA
    }
}

pub fn handler(
    ctx: Context<InitializeExtraAccountMetaList>,
) -> Result<()> {
    // set authority field on white_list account as payer address
    ctx.accounts.white_list.authority = ctx.accounts.payer.key();

    let extra_account_metas = InitializeExtraAccountMetaList::extra_account_metas()?;

    // initialize ExtraAccountMetaList account with extra accounts
    // .map_err() needed because spl-tlv-account-resolution uses solana-program-error 2.x
    // while anchor-lang 1.0 uses 3.x — structurally identical but different semver types
    ExtraAccountMetaList::init::<ExecuteInstruction>(
        &mut ctx.accounts.extra_account_meta_list.try_borrow_mut_data()?,
        &extra_account_metas,
    ).map_err(|_| ProgramError::InvalidAccountData)?;
    Ok(())
}
