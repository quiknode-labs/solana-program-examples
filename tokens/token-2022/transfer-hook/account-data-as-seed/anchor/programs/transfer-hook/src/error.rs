use anchor_lang::prelude::*;

#[error_code]
pub enum TransferError {
    #[msg("The amount is too big")]
    AmountTooBig,
    #[msg("The token is not currently transferring")]
    IsNotCurrentlyTransferring,
}
