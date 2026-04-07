use anchor_lang::prelude::*;
use anchor_spl::token_2022::spl_token_2022::state::AccountState;

// Custom enum to implement AnchorSerialize and AnchorDeserialize
// This is required to pass the enum as an argument to the instruction
#[derive(AnchorSerialize, AnchorDeserialize)]
pub enum AnchorAccountState {
    Uninitialized,
    Initialized,
    Frozen,
}

// Implement conversion from AnchorAccountState to spl_token_2022::state::AccountState
impl AnchorAccountState {
    pub fn to_spl_account_state(&self) -> AccountState {
        match self {
            AnchorAccountState::Uninitialized => AccountState::Uninitialized,
            AnchorAccountState::Initialized => AccountState::Initialized,
            AnchorAccountState::Frozen => AccountState::Frozen,
        }
    }
}
