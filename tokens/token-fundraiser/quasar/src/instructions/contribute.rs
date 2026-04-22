use {
    crate::state::{Contributor, Fundraiser},
    quasar_lang::prelude::*,
    quasar_spl::{Token, TokenCpi},
};

#[derive(Accounts)]
pub struct Contribute {
    #[account(mut)]
    pub contributor: Signer,
    #[account(mut)]
    pub fundraiser: Account<Fundraiser>,
    #[account(mut)]
    pub contributor_account: Account<Contributor>,
    #[account(mut)]
    pub contributor_ta: Account<Token>,
    #[account(mut)]
    pub vault: Account<Token>,
    pub token_program: Program<Token>,
}

impl Contribute {
    #[inline(always)]
    pub fn contribute(&mut self, amount: u64) -> Result<(), ProgramError> {
        require!(amount > 0, ProgramError::InvalidArgument);

        // Transfer tokens from contributor to vault
        self.token_program
            .transfer(&self.contributor_ta, &self.vault, &self.contributor, amount)
            .invoke()?;

        // Update fundraiser state
        self.fundraiser.current_amount = self.fundraiser.current_amount.checked_add(amount)
            .ok_or(ProgramError::ArithmeticOverflow)?;

        // Update contributor tracking
        self.contributor_account.amount = self.contributor_account.amount.checked_add(amount)
            .ok_or(ProgramError::ArithmeticOverflow)?;

        Ok(())
    }
}
