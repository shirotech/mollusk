//! Mollusk errors. These errors will throw a panic. They represent
//! misconfiguration of test inputs or the test environment.

use {solana_pubkey::Pubkey, std::fmt::Display, thiserror::Error};

#[derive(Debug, Error)]
pub enum MolluskError<'a> {
    /// An account required by the instruction was not provided.
    #[error("    [MOLLUSK]: An account required by the instruction was not provided: {0}")]
    AccountMissing(&'a Pubkey),
    /// Program targeted by the instruction is missing from the cache.
    #[error("    [MOLLUSK]: Program targeted by the instruction is missing from the cache: {0}")]
    ProgramNotCached(&'a Pubkey),
    /// Program ID required by the instruction is not mapped in the key map.
    #[error("    [MOLLUSK]: Program ID required by the instruction is not mapped: {0}")]
    ProgramIdNotMapped(&'a Pubkey),
    /// Account index exceeds maximum (255).
    #[error("    [MOLLUSK]: Account index exceeds maximum of 255: {0}")]
    AccountIndexOverflow(usize),
}

pub trait MolluskPanic<T> {
    fn or_panic_with(self, error: MolluskError) -> T;
}

impl<T, E> MolluskPanic<T> for Result<T, E>
where
    E: Display,
{
    fn or_panic_with(self, mollusk_err: MolluskError) -> T {
        self.unwrap_or_else(|err| panic!("{}: {}", mollusk_err, err))
    }
}

impl<T> MolluskPanic<T> for Option<T> {
    fn or_panic_with(self, mollusk_err: MolluskError) -> T {
        self.unwrap_or_else(|| panic!("{}", mollusk_err))
    }
}
