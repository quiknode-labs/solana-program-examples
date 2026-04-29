use quasar_lang::prelude::*;

/// Onchain power status: a single boolean toggle.
/// Derived as a PDA from the seed "power" (single global account).
#[account(discriminator = 1, set_inner)]
#[seeds(b"power")]
pub struct PowerStatus {
    pub is_on: PodBool,
}
