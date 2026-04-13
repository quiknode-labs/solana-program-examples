use anchor_lang::prelude::*;

use anchor_spl::{
    associated_token::AssociatedToken,
    token_interface::{
        close_account, transfer_checked, CloseAccount, Mint, TokenAccount, TokenInterface,
        TransferChecked,
    },
};

use crate::{constants::*, errors::AssetLeasingError, Listing};

#[derive(Accounts)]
pub struct DelistAssetAccountConstraints<'info> {
    #[account(mut)]
    pub owner: Signer<'info>,

    pub asset_mint: InterfaceAccount<'info, Mint>,

    #[account(
        mut,
        associated_token::mint = asset_mint,
        associated_token::authority = owner,
        associated_token::token_program = token_program,
    )]
    pub owner_token_account: InterfaceAccount<'info, TokenAccount>,

    #[account(
        mut,
        close = owner,
        has_one = owner,
        has_one = asset_mint,
        seeds = [LISTING_SEED, owner.key().as_ref(), asset_mint.key().as_ref()],
        bump = listing.bump,
    )]
    pub listing: Account<'info, Listing>,

    #[account(
        mut,
        associated_token::mint = asset_mint,
        associated_token::authority = listing,
        associated_token::token_program = token_program,
    )]
    pub vault: InterfaceAccount<'info, TokenAccount>,

    pub associated_token_program: Program<'info, AssociatedToken>,
    pub token_program: Interface<'info, TokenInterface>,
    pub system_program: Program<'info, System>,
}

pub fn handle_delist_asset(context: Context<DelistAssetAccountConstraints>) -> Result<()> {
    require!(
        !context.accounts.listing.active_lease,
        AssetLeasingError::AssetCurrentlyLeased
    );

    let owner_key = context.accounts.owner.key();
    let asset_mint_key = context.accounts.asset_mint.key();
    let listing_bump = context.accounts.listing.bump;

    let seeds = &[
        LISTING_SEED,
        owner_key.as_ref(),
        asset_mint_key.as_ref(),
        &[listing_bump],
    ];
    let signer_seeds = [&seeds[..]];

    // Transfer everything from vault back to owner
    let transfer_accounts = TransferChecked {
        from: context.accounts.vault.to_account_info(),
        mint: context.accounts.asset_mint.to_account_info(),
        to: context.accounts.owner_token_account.to_account_info(),
        authority: context.accounts.listing.to_account_info(),
    };

    let cpi_context = CpiContext::new_with_signer(
        context.accounts.token_program.key(),
        transfer_accounts,
        &signer_seeds,
    );

    transfer_checked(
        cpi_context,
        context.accounts.vault.amount,
        context.accounts.asset_mint.decimals,
    )?;

    // Close the vault ATA and reclaim rent to owner
    let close_accounts = CloseAccount {
        account: context.accounts.vault.to_account_info(),
        destination: context.accounts.owner.to_account_info(),
        authority: context.accounts.listing.to_account_info(),
    };

    let cpi_context = CpiContext::new_with_signer(
        context.accounts.token_program.key(),
        close_accounts,
        &signer_seeds,
    );

    close_account(cpi_context)
}
