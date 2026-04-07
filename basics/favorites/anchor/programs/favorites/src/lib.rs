pub mod constants;
pub mod instructions;
pub mod state;

use anchor_lang::prelude::*;

pub use instructions::*;
pub use state::*;

// Our program's address!
// This matches the key in the target/deploy directory
declare_id!("4wHiZQvMXQKaF1BbgN9Y9D5g1dZersK3J7mLGG5cUjEm");

// Our Solana program!
#[program]
pub mod favorites {
    use super::*;

    pub fn set_favorites(
        context: Context<SetFavorites>,
        number: u64,
        color: String,
        hobbies: Vec<String>,
    ) -> Result<()> {
        set_favorites::handler(context, number, color, hobbies)
    }
}
