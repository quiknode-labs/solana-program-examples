use {
    crate::state::Fundraiser,
    quasar_lang::prelude::*,
    quasar_spl::{Token, TokenCpi},
};

#[derive(Accounts)]
pub struct CheckContributions {
    #[account(mut)]
    pub maker: Signer,
    #[account(
        mut,
        has_one = maker,
        close = maker,
        seeds = Fundraiser::seeds(maker),
        bump = fundraiser.bump
    )]
    pub fundraiser: Account<Fundraiser>,
    #[account(mut)]
    pub vault: Account<Token>,
    #[account(mut)]
    pub maker_ta: Account<Token>,
    pub token_program: Program<Token>,
}

#[inline(always)]
pub fn handle_check_contributions(accounts: &mut CheckContributions, bumps: &CheckContributionsBumps) -> Result<(), ProgramError> {
    // Verify the target was met
    require!(
        accounts.fundraiser.current_amount >= accounts.fundraiser.amount_to_raise,
        ProgramError::Custom(0) // TargetNotMet
    );

    let seeds = accounts.fundraiser_seeds(bumps);

    // Transfer all vault funds to the maker
    let vault_amount = accounts.vault.amount();
    accounts.token_program
        .transfer(&accounts.vault, &accounts.maker_ta, &accounts.fundraiser, vault_amount)
        .invoke_signed(&seeds)?;

    // Close the vault token account
    accounts.token_program
        .close_account(&accounts.vault, &accounts.maker, &accounts.fundraiser)
        .invoke_signed(&seeds)?;

    Ok(())
}
