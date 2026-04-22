use {
    crate::state::Favorites,
    quasar_lang::prelude::*,
};

/// Accounts for setting user favourites. Uses `init_if_needed` so the same
/// instruction can create or update the favourites PDA.
#[derive(Accounts)]
pub struct SetFavorites {
    #[account(mut)]
    pub user: Signer,
    #[account(mut, init_if_needed, payer = user, seeds = Favorites::seeds(user), bump)]
    pub favorites: Account<Favorites<'_>>,
    pub system_program: Program<System>,
}

impl SetFavorites {
    #[inline(always)]
    pub fn set_favorites(&mut self, number: u64, color: &str) -> Result<(), ProgramError> {
        self.favorites.set_inner(
            number,
            color,
            self.user.to_account_view(),
            None,
        )
    }
}
