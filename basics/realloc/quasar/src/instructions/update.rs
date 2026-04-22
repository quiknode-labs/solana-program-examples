use {
    crate::state::MessageAccount,
    quasar_lang::prelude::*,
};

/// Accounts for updating a message account.
/// Quasar's `set_inner` automatically handles realloc when the new message
/// is longer than the current account data. No explicit realloc needed.
#[derive(Accounts)]
pub struct Update {
    #[account(mut)]
    pub payer: Signer,
    #[account(mut)]
    pub message_account: Account<MessageAccount<'_>>,
    pub system_program: Program<System>,
}

impl Update {
    #[inline(always)]
    pub fn update(&mut self, message: &str) -> Result<(), ProgramError> {
        self.message_account.set_inner(
            message,
            self.payer.to_account_view(),
            None,
        )
    }
}
