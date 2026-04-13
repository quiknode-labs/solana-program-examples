use anchor_lang::prelude::*;
use anchor_lang::system_program;

use anchor_spl::{
    associated_token::AssociatedToken,
    token_interface::{transfer_checked, Mint, TokenAccount, TokenInterface, TransferChecked},
};

use crate::{constants::*, errors::AssetLeasingError, Lease, LeaseConfig, Listing};

#[derive(Accounts)]
pub struct RentAssetAccountConstraints<'info> {
    #[account(mut)]
    pub renter: Signer<'info>,

    #[account(mut)]
    pub owner: SystemAccount<'info>,

    pub asset_mint: InterfaceAccount<'info, Mint>,

    #[account(
        seeds = [LEASE_CONFIG_SEED],
        bump = lease_config.bump,
    )]
    pub lease_config: Account<'info, LeaseConfig>,

    #[account(
        mut,
        address = lease_config.authority,
    )]
    pub fee_collector: SystemAccount<'info>,

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

    /// The renter's ATA to receive the leased asset
    #[account(
        init_if_needed,
        payer = renter,
        associated_token::mint = asset_mint,
        associated_token::authority = renter,
        associated_token::token_program = token_program,
    )]
    pub renter_token_account: InterfaceAccount<'info, TokenAccount>,

    #[account(
        init,
        payer = renter,
        space = ANCHOR_DISCRIMINATOR + Lease::INIT_SPACE,
        seeds = [LEASE_SEED, listing.key().as_ref(), renter.key().as_ref()],
        bump,
    )]
    pub lease: Account<'info, Lease>,

    pub associated_token_program: Program<'info, AssociatedToken>,
    pub token_program: Interface<'info, TokenInterface>,
    pub system_program: Program<'info, System>,
}

pub fn handle_rent_asset(
    context: Context<RentAssetAccountConstraints>,
    duration: i64,
) -> Result<()> {
    let listing = &context.accounts.listing;

    require!(
        duration >= listing.min_duration,
        AssetLeasingError::DurationTooShort
    );
    require!(
        duration <= listing.max_duration,
        AssetLeasingError::DurationTooLong
    );

    // Calculate total cost: price_per_second * duration
    let total_cost = listing
        .price_per_second
        .checked_mul(duration as u64)
        .ok_or(AssetLeasingError::ArithmeticOverflow)?;

    // Calculate program fee
    let fee_amount = total_cost
        .checked_mul(context.accounts.lease_config.fee_basis_points as u64)
        .ok_or(AssetLeasingError::ArithmeticOverflow)?
        .checked_div(MAX_FEE_BASIS_POINTS as u64)
        .ok_or(AssetLeasingError::ArithmeticOverflow)?;

    // Owner receives total minus fee
    let owner_amount = total_cost
        .checked_sub(fee_amount)
        .ok_or(AssetLeasingError::ArithmeticOverflow)?;

    // Pay the owner
    system_program::transfer(
        CpiContext::new(
            context.accounts.system_program.key(),
            system_program::Transfer {
                from: context.accounts.renter.to_account_info(),
                to: context.accounts.owner.to_account_info(),
            },
        ),
        owner_amount,
    )?;

    // Pay the program fee
    if fee_amount > 0 {
        system_program::transfer(
            CpiContext::new(
                context.accounts.system_program.key(),
                system_program::Transfer {
                    from: context.accounts.renter.to_account_info(),
                    to: context.accounts.fee_collector.to_account_info(),
                },
            ),
            fee_amount,
        )?;
    }

    // Transfer asset from vault to renter
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

    let transfer_accounts = TransferChecked {
        from: context.accounts.vault.to_account_info(),
        mint: context.accounts.asset_mint.to_account_info(),
        to: context.accounts.renter_token_account.to_account_info(),
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

    // Record the lease
    let clock = Clock::get()?;
    let start_time = clock.unix_timestamp;
    let end_time = start_time
        .checked_add(duration)
        .ok_or(AssetLeasingError::ArithmeticOverflow)?;

    context.accounts.lease.set_inner(Lease {
        renter: context.accounts.renter.key(),
        listing: context.accounts.listing.key(),
        start_time,
        end_time,
        returned: false,
        bump: context.bumps.lease,
    });

    // Mark the listing as actively leased
    context.accounts.listing.active_lease = true;

    Ok(())
}
