use anchor_lang::prelude::*;

#[error_code]
pub enum ErrorCode {
    #[msg("Invalid Ethereum signature")]
    InvalidSignature,
}
