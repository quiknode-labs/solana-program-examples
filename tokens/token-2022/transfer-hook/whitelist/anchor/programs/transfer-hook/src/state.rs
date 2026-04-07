use anchor_lang::prelude::*;

#[account]
pub struct WhiteList {
    pub authority: Pubkey,
    pub white_list: Vec<Pubkey>,
}
