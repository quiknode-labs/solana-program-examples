use quasar_lang::prelude::*;

/// Onchain counter account.
#[account(discriminator = 1)]
#[seeds(b"counter", payer: Address)]
pub struct Counter {
    pub count: u64,
}
