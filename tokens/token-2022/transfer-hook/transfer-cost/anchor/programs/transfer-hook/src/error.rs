use anchor_lang::prelude::*;

#[error_code]
pub enum TransferError {
    #[msg("Amount Too big")]
    AmountTooBig,
    #[msg("The token is not currently transferring")]
    IsNotCurrentlyTransferring,
}
