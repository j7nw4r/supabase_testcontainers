/*! A library for managing Supabase containers in tests.

This crate provides utilities for setting up and managing Supabase services
in a containerized environment, primarily for testing purposes.
*/

#[cfg(feature = "analytics")]
pub use analytics::Analytics;
#[cfg(feature = "auth")]
pub use auth::Auth;
#[cfg(feature = "const")]
pub use consts::*;
#[cfg(feature = "functions")]
pub use functions::Functions;
#[cfg(feature = "graphql")]
pub use graphql::GraphQL;
#[cfg(feature = "postgrest")]
pub use postgrest::PostgREST;
#[cfg(feature = "realtime")]
pub use realtime::Realtime;
#[cfg(feature = "storage")]
pub use storage::Storage;

#[cfg(feature = "analytics")]
mod analytics;
#[cfg(feature = "auth")]
mod auth;
#[cfg(feature = "const")]
mod consts;
#[cfg(feature = "error")]
mod error;
#[cfg(feature = "functions")]
mod functions;
#[cfg(feature = "graphql")]
mod graphql;
#[cfg(feature = "postgrest")]
mod postgrest;
#[cfg(feature = "realtime")]
mod realtime;
#[cfg(feature = "storage")]
mod storage;
