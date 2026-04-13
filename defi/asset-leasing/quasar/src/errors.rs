use quasar_lang::prelude::ProgramError;

/// Custom error codes for the asset leasing protocol.
/// Offset from 6000 to avoid collision with built-in errors.
#[repr(u32)]
pub enum AssetLeasingError {
    FeeTooHigh = 6000,
    DurationTooShort = 6001,
    DurationTooLong = 6002,
    InvalidDurationRange = 6003,
    AssetCurrentlyLeased = 6004,
    LeaseNotExpired = 6005,
    LeaseAlreadyReturned = 6006,
    NotTheRenter = 6007,
    InvalidPrice = 6008,
    ArithmeticOverflow = 6009,
}

impl From<AssetLeasingError> for ProgramError {
    fn from(e: AssetLeasingError) -> Self {
        ProgramError::Custom(e as u32)
    }
}
