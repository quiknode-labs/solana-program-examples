use anchor_lang::prelude::*;

use anchor_spl::token_interface::{
    transfer_checked, Mint, TokenAccount, TokenInterface, TransferChecked,
};

use crate::{constants::*, errors::AssetLeasingError, Lease, Listing};

#[derive(Accounts)]
pub struct ReturnAssetAccountConstraints<'info> {
    #[account(mut)]
    pub renter: Signer<'info>,

    pub asset_mint: InterfaceAccount<'info, Mint>,

    /// CHECK: validated via listing.owner — needed for PDA derivation
    pub owner: UncheckedAccount<'info>,

    #[account(
        mut,
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

    #[account(
        mut,
        associated_token::mint = asset_mint,
        associated_token::authority = renter,
        associated_token::token_program = token_program,
    )]
    pub renter_token_account: InterfaceAccount<'info, TokenAccount>,

    #[account(
        mut,
        has_one = renter @ AssetLeasingError::NotTheRenter,
        constraint = lease.listing == listing.key(),
        seeds = [LEASE_SEED, listing.key().as_ref(), renter.key().as_ref()],
        bump = lease.bump,
    )]
    pub lease: Account<'info, Lease>,

    pub token_program: Interface<'info, TokenInterface>,
}

pub fn handle_return_asset(context: Context<ReturnAssetAccountConstraints>) -> Result<()> {
    require!(
        !context.accounts.lease.returned,
        AssetLeasingError::LeaseAlreadyReturned
    );

    // Transfer asset from renter back to vault
    let transfer_accounts = TransferChecked {
        from: context.accounts.renter_token_account.to_account_info(),
        mint: context.accounts.asset_mint.to_account_info(),
        to: context.accounts.vault.to_account_info(),
        authority: context.accounts.renter.to_account_info(),
    };

    let cpi_context = CpiContext::new(
        context.accounts.token_program.key(),
        transfer_accounts,
    );

    // Transfer all tokens back — for NFTs this is 1, for fungible it's the full amount
    transfer_checked(
        cpi_context,
        context.accounts.renter_token_account.amount,
        context.accounts.asset_mint.decimals,
    )?;

    // Mark lease as returned and listing as available
    context.accounts.lease.returned = true;
    context.accounts.listing.active_lease = false;

    Ok(())
}
