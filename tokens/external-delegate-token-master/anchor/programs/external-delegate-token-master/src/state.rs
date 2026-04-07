use anchor_lang::prelude::*;

#[account]
pub struct UserAccount {
    pub authority: Pubkey,
    pub ethereum_address: [u8; 20],
}
