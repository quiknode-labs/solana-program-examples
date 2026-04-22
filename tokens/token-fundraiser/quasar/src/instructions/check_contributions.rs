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

impl CheckContributions {
    #[inline(always)]
    pub fn check_contributions(&mut self, bumps: &CheckContributionsBumps) -> Result<(), ProgramError> {
        // Verify the target was met
        require!(
            self.fundraiser.current_amount >= self.fundraiser.amount_to_raise,
            ProgramError::Custom(0) // TargetNotMet
        );

        let seeds = self.fundraiser_seeds(bumps);

        // Transfer all vault funds to the maker
        let vault_amount = self.vault.amount();
        self.token_program
            .transfer(&self.vault, &self.maker_ta, &self.fundraiser, vault_amount)
            .invoke_signed(&seeds)?;

        // Close the vault token account
        self.token_program
            .close_account(&self.vault, &self.maker, &self.fundraiser)
            .invoke_signed(&seeds)?;

        Ok(())
    }
}
