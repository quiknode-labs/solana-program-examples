use {
    crate::state::MessageAccount,
    quasar_lang::prelude::*,
};

/// Accounts for initialising a new message account.
/// The message_account is a random keypair (not a PDA) — same as the Anchor version.
#[derive(Accounts)]
pub struct Initialize {
    #[account(mut)]
    pub payer: Signer,
    #[account(mut, init, payer = payer)]
    pub message_account: Account<MessageAccount<'_>>,
    pub system_program: Program<System>,
}

impl Initialize {
    #[inline(always)]
    pub fn initialize(&mut self, message: &str) -> Result<(), ProgramError> {
        self.message_account.set_inner(
            message,
            self.payer.to_account_view(),
            None,
        )
    }
}
