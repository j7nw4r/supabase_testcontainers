use thiserror::Error;

/// Errors that can occur during Supabase container operations
#[derive(Debug, Error, Default)]
pub enum Error {
    /// Represents an unknown or unspecified error condition
    #[error("unknown error")]
    #[default]
    Unknown
}