use anchor_lang::prelude::*;
use anchor_spl::{
    associated_token::AssociatedToken,
    token::Token,
    token_interface::Mint,
};
use spl_tlv_account_resolution::{
    account::ExtraAccountMeta, seeds::Seed, state::ExtraAccountMetaList,
};
use spl_discriminator::SplDiscriminate;
use spl_transfer_hook_interface::instruction::ExecuteInstruction;

use crate::state::CounterAccount;

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
    #[account(init, seeds = [b"counter"], bump, payer = payer, space = 9)]
    pub counter_account: Account<'info, CounterAccount>,
    pub system_program: Program<'info, System>,
}

// Define extra account metas to store on extra_account_meta_list account
impl<'info> InitializeExtraAccountMetaList<'info> {
    pub fn extra_account_metas() -> Result<Vec<ExtraAccountMeta>> {
        // When the token2022 program CPIs to the transfer_hook instruction on this program,
        // the accounts are provided in order defined specified the list:

        // index 0-3 are the accounts required for token transfer (source, mint, destination, owner)
        // index 4 is address of ExtraAccountMetaList account

        let wsol_mint = anchor_lang::solana_program::pubkey::Pubkey::from_str_const("So11111111111111111111111111111111111111112");
        let token_program_id = Token::id();
        let ata_program_id = AssociatedToken::id();

        Ok(vec![
            // index 5, wrapped SOL mint
            ExtraAccountMeta::new_with_pubkey(&wsol_mint, false, false)
                .map_err(|_| ProgramError::InvalidArgument)?,
            // index 6, token program (for wsol token transfer)
            ExtraAccountMeta::new_with_pubkey(&token_program_id, false, false)
                .map_err(|_| ProgramError::InvalidArgument)?,
            // index 7, associated token program
            ExtraAccountMeta::new_with_pubkey(&ata_program_id, false, false)
                .map_err(|_| ProgramError::InvalidArgument)?,
            // index 8, delegate PDA
            ExtraAccountMeta::new_with_seeds(
                &[Seed::Literal {
                    bytes: b"delegate".to_vec(),
                }],
                false, // is_signer
                true,  // is_writable
            )
            .map_err(|_| ProgramError::InvalidArgument)?,
            // index 9, delegate wrapped SOL token account
            ExtraAccountMeta::new_external_pda_with_seeds(
                7, // associated token program index
                &[
                    Seed::AccountKey { index: 8 }, // owner index (delegate PDA)
                    Seed::AccountKey { index: 6 }, // token program index
                    Seed::AccountKey { index: 5 }, // wsol mint index
                ],
                false, // is_signer
                true,  // is_writable
            )
            .map_err(|_| ProgramError::InvalidArgument)?,
            // index 10, sender wrapped SOL token account
            ExtraAccountMeta::new_external_pda_with_seeds(
                7, // associated token program index
                &[
                    Seed::AccountKey { index: 3 }, // owner index
                    Seed::AccountKey { index: 6 }, // token program index
                    Seed::AccountKey { index: 5 }, // wsol mint index
                ],
                false, // is_signer
                true,  // is_writable
            )
            .map_err(|_| ProgramError::InvalidArgument)?,
            ExtraAccountMeta::new_with_seeds(
                &[Seed::Literal {
                    bytes: b"counter".to_vec(),
                }],
                false, // is_signer
                true,  // is_writable
            )
            .map_err(|_| ProgramError::InvalidArgument)?,
        ])
    }

    /// Returns the count of extra account metas (avoids the error conversion issue in #[account] attributes)
    pub fn extra_account_metas_count() -> usize {
        7 // wsol_mint, token_program, ata_program, delegate, delegate_wsol, sender_wsol, counter
    }
}

pub fn handler(
    ctx: Context<InitializeExtraAccountMetaList>,
) -> Result<()> {
    let extra_account_metas = InitializeExtraAccountMetaList::extra_account_metas()?;

    // initialize ExtraAccountMetaList account with extra accounts
    ExtraAccountMetaList::init::<ExecuteInstruction>(
        &mut ctx.accounts.extra_account_meta_list.try_borrow_mut_data()?,
        &extra_account_metas,
    )
    .map_err(|_| ProgramError::InvalidAccountData)?;

    Ok(())
}
