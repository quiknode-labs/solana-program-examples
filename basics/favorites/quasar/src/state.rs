use quasar_lang::prelude::*;

/// User favourites stored onchain.
///
/// The Anchor version also stores `hobbies: Vec<String>`, but Quasar doesn't
/// support nested dynamic types (Vec<String>). We keep number + color, which
/// demonstrates fixed + dynamic field mixing in Quasar.
#[account(discriminator = 1, set_inner)]
#[seeds(b"favorites", user: Address)]
pub struct Favorites {
    pub number: u64,
    pub color: String<50>,
}
