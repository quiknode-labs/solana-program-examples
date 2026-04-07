use anchor_lang::prelude::*;

#[account]
pub struct PowerStatus {
    pub is_on: bool,
}
