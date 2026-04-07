use anchor_lang::prelude::*;

use crate::constants::ANCHOR_DISCRIMINATOR_SIZE;
use crate::state::Favorites;

// When people call the set_favorites instruction, they will need to provide the accounts that will be modifed. This keeps Solana fast!
#[derive(Accounts)]
pub struct SetFavorites<'info> {
    #[account(mut)]
    pub user: Signer<'info>,

    #[account(
        init_if_needed,
        payer = user,
        space = ANCHOR_DISCRIMINATOR_SIZE + Favorites::INIT_SPACE,
        seeds=[b"favorites", user.key().as_ref()],
        bump
    )]
    pub favorites: Account<'info, Favorites>,

    pub system_program: Program<'info, System>,
}

// Our instruction handler! It sets the user's favorite number and color
pub fn handler(
    context: Context<SetFavorites>,
    number: u64,
    color: String,
    hobbies: Vec<String>,
) -> Result<()> {
    msg!("Greetings from {}", context.program_id);
    let user_public_key = context.accounts.user.key();
    msg!(
        "User {user_public_key}'s favorite number is {number}, favorite color is: {color}, and their hobbies are {hobbies:?}",
    );

    context.accounts.favorites.set_inner(Favorites {
        number,
        color,
        hobbies,
    });
    Ok(())
}

// We can also add a get_favorites instruction handler to return the user's favorite number and color
