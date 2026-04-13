use anchor_lang::prelude::*;

use anchor_spl::{
    associated_token::AssociatedToken,
    token_interface::{transfer_checked, Mint, TokenAccount, TokenInterface, TransferChecked},
};

use crate::{constants::*, errors::AssetLeasingError, Listing};

#[derive(Accounts)]
pub struct ListAssetAccountConstraints<'info> {
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
        init,
        payer = owner,
        space = ANCHOR_DISCRIMINATOR + Listing::INIT_SPACE,
        seeds = [LISTING_SEED, owner.key().as_ref(), asset_mint.key().as_ref()],
        bump
    )]
    pub listing: Account<'info, Listing>,

    /// The vault holds the listed asset while it's available or leased.
    /// Using an ATA owned by the listing PDA so the program can sign transfers.
    #[account(
        init,
        payer = owner,
        associated_token::mint = asset_mint,
        associated_token::authority = listing,
        associated_token::token_program = token_program,
    )]
    pub vault: InterfaceAccount<'info, TokenAccount>,

    pub associated_token_program: Program<'info, AssociatedToken>,
    pub token_program: Interface<'info, TokenInterface>,
    pub system_program: Program<'info, System>,
}

pub fn handle_list_asset(
    context: Context<ListAssetAccountConstraints>,
    price_per_second: u64,
    min_duration: i64,
    max_duration: i64,
    amount: u64,
) -> Result<()> {
    require!(price_per_second > 0, AssetLeasingError::InvalidPrice);
    require!(
        min_duration <= max_duration,
        AssetLeasingError::InvalidDurationRange
    );

    // Transfer the asset from owner to vault
    let transfer_accounts = TransferChecked {
        from: context.accounts.owner_token_account.to_account_info(),
        mint: context.accounts.asset_mint.to_account_info(),
        to: context.accounts.vault.to_account_info(),
        authority: context.accounts.owner.to_account_info(),
    };

    let cpi_context = CpiContext::new(
        context.accounts.token_program.key(),
        transfer_accounts,
    );

    transfer_checked(cpi_context, amount, context.accounts.asset_mint.decimals)?;

    context.accounts.listing.set_inner(Listing {
        owner: context.accounts.owner.key(),
        asset_mint: context.accounts.asset_mint.key(),
        price_per_second,
        min_duration,
        max_duration,
        active_lease: false,
        bump: context.bumps.listing,
    });

    Ok(())
}
