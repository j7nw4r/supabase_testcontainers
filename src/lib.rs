/*! A library for managing Supabase containers in tests.

This crate provides utilities for setting up and managing Supabase services
in a containerized environment, primarily for testing purposes.
*/

#[cfg(feature = "auth")]
pub use auth::Auth;
#[cfg(feature = "const")]
pub use consts::*;

#[cfg(feature = "auth")]
/// Authentication module for Supabase services
mod auth;
#[cfg(feature = "const")]
/// Constants used throughout the crate
mod consts;
/// Error types and implementations
mod error;
