use anchor_lang::prelude::*;

#[error_code]
pub enum AssetLeasingError {
    #[msg("Fee basis points must not exceed 10,000 (100%)")]
    FeeTooHigh,

    #[msg("Duration is below the listing minimum")]
    DurationTooShort,

    #[msg("Duration exceeds the listing maximum")]
    DurationTooLong,

    #[msg("Minimum duration must be less than or equal to maximum duration")]
    InvalidDurationRange,

    #[msg("Cannot delist an asset that is currently leased")]
    AssetCurrentlyLeased,

    #[msg("Lease has not expired yet")]
    LeaseNotExpired,

    #[msg("Lease has already been returned")]
    LeaseAlreadyReturned,

    #[msg("Only the renter can return the asset")]
    NotTheRenter,

    #[msg("Price per second must be greater than zero")]
    InvalidPrice,

    #[msg("Arithmetic overflow")]
    ArithmeticOverflow,
}
