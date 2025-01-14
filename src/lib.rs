#[cfg(feature = "auth")]
pub use auth::Auth;
pub use consts::DOCKER_INTERNAL_HOST;
pub use consts::LOCAL_HOST;

#[cfg(feature = "auth")]
mod auth;
mod consts;
