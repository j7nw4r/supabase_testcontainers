/*! Supabase Auth container management module.

This module provides a testcontainer implementation for [Supabase Auth](https://github.com/supabase/auth)
(GoTrue), enabling integration testing with a real Auth service.

# Features

- Full configuration via fluent builder API
- Automatic database schema initialization
- Support for email/password, anonymous, and OAuth authentication
- Test-friendly defaults (autoconfirm enabled, debug logging)

# Example

```rust,no_run
use supabase_testcontainers_modules::{Auth, AUTH_PORT, DOCKER_INTERNAL_HOST, LOCAL_HOST};
use testcontainers::runners::AsyncRunner;
use testcontainers_modules::postgres::Postgres;

#[tokio::main]
async fn main() -> anyhow::Result<()> {
    // 1. Start PostgreSQL
    let postgres = Postgres::default().start().await?;
    let pg_port = postgres.get_host_port_ipv4(5432).await?;

    // 2. Configure connection strings
    let auth_db_url = format!(
        "postgres://supabase_auth_admin:password@{}:{}/postgres",
        DOCKER_INTERNAL_HOST, pg_port
    );
    let local_db_url = format!(
        "postgres://postgres:postgres@{}:{}/postgres",
        LOCAL_HOST, pg_port
    );

    // 3. Initialize and start Auth
    let auth = Auth::default()
        .with_db_url(&auth_db_url)
        .with_anonymous_users(true)
        .init_db_schema(&local_db_url, "password")
        .await?
        .start()
        .await?;

    let auth_port = auth.get_host_port_ipv4(AUTH_PORT).await?;
    println!("Auth running at http://localhost:{}", auth_port);

    Ok(())
}
```

# Configuration

The [`Auth`] struct provides builder methods for common configuration options:

- [`Auth::with_db_url`] - PostgreSQL connection string
- [`Auth::with_jwt_secret`] - JWT signing secret
- [`Auth::with_signup_disabled`] - Disable user registration
- [`Auth::with_anonymous_users`] - Enable anonymous authentication
- [`Auth::with_mailer_autoconfirm`] - Skip email verification (testing)

See the struct documentation for the full list of options.
*/

use std::borrow::Cow;
use std::collections::BTreeMap;

use anyhow::{bail, Context};
use testcontainers_modules::testcontainers::core::{
    ContainerPort, ContainerState, ExecCommand, WaitFor,
};
use testcontainers_modules::testcontainers::{Image, TestcontainersError};
use tokio_postgres::NoTls;

/// Default image name for Supabase Auth
const NAME: &str = "supabase/gotrue";
/// Default image tag version
const TAG: &str = "v2.183.0";
/// Default port for Supabase Auth API
pub const AUTH_PORT: u16 = 9999;

#[cfg(feature = "auth")]
/// Supabase Auth (GoTrue) container for integration testing.
///
/// This struct implements the [`Image`] trait from testcontainers, allowing you to
/// start a fully configured Supabase Auth service for testing authentication flows.
///
/// # Default Configuration
///
/// The default configuration includes:
/// - PostgreSQL driver enabled
/// - Auth schema namespace set to "auth"
/// - Email and SMS autoconfirm enabled (for testing convenience)
/// - Anonymous users enabled
/// - Debug logging
///
/// # Example
///
/// ```rust,no_run
/// use supabase_testcontainers_modules::Auth;
///
/// let auth = Auth::default()
///     .with_db_url("postgres://user:pass@host:5432/db")
///     .with_jwt_secret("my-32-char-secret-for-jwt-signing")
///     .with_signup_disabled(false);
/// ```
#[derive(Debug, Clone)]
pub struct Auth {
    /// Environment variables to be passed to the container
    env_vars: BTreeMap<String, String>,
    /// Docker image tag version
    tag: String,
}

impl Auth {
    /// Creates a new Auth instance with the specified PostgreSQL connection string
    pub fn new(postgres_connection_string: impl Into<String>) -> Self {
        Self::default().with_db_url(postgres_connection_string)
    }

    /// Sets the PostgreSQL database connection URL
    pub fn with_db_url(mut self, url: impl Into<String>) -> Self {
        self.env_vars.insert("DATABASE_URL".to_string(), url.into());
        self
    }

    /// Sets the JWT secret for token signing (min 32 chars recommended)
    pub fn with_jwt_secret(mut self, secret: impl Into<String>) -> Self {
        self.env_vars
            .insert("GOTRUE_JWT_SECRET".to_string(), secret.into());
        self
    }

    /// Sets the JWT token expiration time in seconds
    pub fn with_jwt_expiry(mut self, seconds: u32) -> Self {
        self.env_vars
            .insert("GOTRUE_JWT_EXP".to_string(), seconds.to_string());
        self
    }

    /// Sets the external API URL
    pub fn with_api_external_url(mut self, url: impl Into<String>) -> Self {
        self.env_vars
            .insert("API_EXTERNAL_URL".to_string(), url.into());
        self
    }

    /// Sets the site URL (frontend application URL)
    pub fn with_site_url(mut self, url: impl Into<String>) -> Self {
        self.env_vars
            .insert("GOTRUE_SITE_URL".to_string(), url.into());
        self
    }

    /// Enables or disables user signups
    pub fn with_signup_disabled(mut self, disabled: bool) -> Self {
        self.env_vars
            .insert("GOTRUE_DISABLE_SIGNUP".to_string(), disabled.to_string());
        self
    }

    /// Enables or disables anonymous user authentication
    pub fn with_anonymous_users(mut self, enabled: bool) -> Self {
        self.env_vars.insert(
            "GOTRUE_EXTERNAL_ANONYMOUS_USERS_ENABLED".to_string(),
            enabled.to_string(),
        );
        self
    }

    /// Enables or disables automatic email confirmation (useful for testing)
    pub fn with_mailer_autoconfirm(mut self, enabled: bool) -> Self {
        self.env_vars
            .insert("GOTRUE_MAILER_AUTOCONFIRM".to_string(), enabled.to_string());
        self
    }

    /// Enables or disables automatic SMS confirmation (useful for testing)
    pub fn with_sms_autoconfirm(mut self, enabled: bool) -> Self {
        self.env_vars
            .insert("GOTRUE_SMS_AUTOCONFIRM".to_string(), enabled.to_string());
        self
    }

    /// Sets the log level (debug, info, warn, error)
    pub fn with_log_level(mut self, level: impl Into<String>) -> Self {
        self.env_vars
            .insert("GOTRUE_LOG_LEVEL".to_string(), level.into());
        self
    }

    /// Sets a custom Docker image tag/version
    pub fn with_tag(mut self, tag: impl Into<String>) -> Self {
        self.tag = tag.into();
        self
    }

    /// Adds a custom environment variable
    pub fn with_env(mut self, key: impl Into<String>, value: impl Into<String>) -> Self {
        self.env_vars.insert(key.into(), value.into());
        self
    }

    /// Returns the Git release version string based on the current tag
    pub fn git_release_version(&self) -> String {
        let version = self.tag[1..].to_string();
        format!("release/{}", version)
    }

    /// Initializes the database schema for Supabase Auth
    ///
    /// Creates the required PostgreSQL users and schema for Supabase Auth to function.
    /// This should be called before starting the Auth container.
    ///
    /// # Arguments
    /// * `db_url` - PostgreSQL connection string (e.g., "postgres://postgres:postgres@localhost:5432/postgres")
    /// * `auth_admin_password` - Password for the supabase_auth_admin user
    ///
    /// # Returns
    /// * `anyhow::Result<Self>` - The Auth instance with initialized schema
    ///
    /// # Errors
    /// Returns an error if:
    /// * The database URL is empty
    /// * Database connection fails
    /// * Schema creation fails
    pub async fn init_db_schema(
        self,
        db_url: &str,
        auth_admin_password: &str,
    ) -> anyhow::Result<Self> {
        if db_url.is_empty() {
            bail!("database URL cannot be empty");
        }

        let db_schema = self
            .env_vars
            .get("DB_NAMESPACE")
            .map(|s| s.as_str())
            .unwrap_or("auth");

        let (client, connection) = tokio_postgres::connect(db_url, NoTls)
            .await
            .with_context(|| format!("failed to connect to PostgreSQL at {}", db_url))?;

        // Spawn connection handler
        tokio::spawn(async move {
            if let Err(e) = connection.await {
                eprintln!("PostgreSQL connection error: {}", e);
            }
        });

        let query = format!(
            "CREATE USER supabase_admin LOGIN CREATEROLE CREATEDB REPLICATION BYPASSRLS;
            CREATE USER supabase_auth_admin NOINHERIT CREATEROLE LOGIN NOREPLICATION PASSWORD '{auth_admin_password}';
            CREATE SCHEMA IF NOT EXISTS {db_schema} AUTHORIZATION supabase_auth_admin;
            GRANT CREATE ON DATABASE postgres TO supabase_auth_admin;
            ALTER USER supabase_auth_admin SET search_path = '{db_schema}';"
        );

        client
            .batch_execute(&query)
            .await
            .context("failed to initialize auth database schema")?;

        Ok(self)
    }
}

/// Default implementation for Auth container configuration
impl Default for Auth {
    /// Creates a default Auth instance with pre-configured environment variables
    ///
    /// Sets up essential defaults for:
    /// * Database driver and namespace
    /// * JWT configuration (with test-safe secret)
    /// * API host and port bindings
    /// * Test conveniences (autoconfirm enabled, debug logging)
    fn default() -> Self {
        let mut env_vars = BTreeMap::new();

        // Database configuration
        env_vars.insert("GOTRUE_DB_DRIVER".to_string(), "postgres".to_string());
        env_vars.insert("DB_NAMESPACE".to_string(), "auth".to_string());

        // JWT configuration (32+ char secret for security)
        env_vars.insert(
            "GOTRUE_JWT_SECRET".to_string(),
            "super-secret-jwt-token-for-testing-at-least-32-chars".to_string(),
        );
        env_vars.insert("GOTRUE_JWT_EXP".to_string(), "3600".to_string());

        // API configuration
        env_vars.insert("GOTRUE_API_HOST".to_string(), "0.0.0.0".to_string());
        env_vars.insert("PORT".to_string(), "9999".to_string());
        env_vars.insert(
            "API_EXTERNAL_URL".to_string(),
            "http://localhost:9999".to_string(),
        );
        env_vars.insert(
            "GOTRUE_SITE_URL".to_string(),
            "http://localhost:3000".to_string(),
        );

        // Auth settings
        env_vars.insert("GOTRUE_DISABLE_SIGNUP".to_string(), "false".to_string());
        env_vars.insert(
            "GOTRUE_EXTERNAL_ANONYMOUS_USERS_ENABLED".to_string(),
            "true".to_string(),
        );

        // Test conveniences (autoconfirm to skip email/sms verification)
        env_vars.insert("GOTRUE_MAILER_AUTOCONFIRM".to_string(), "true".to_string());
        env_vars.insert("GOTRUE_SMS_AUTOCONFIRM".to_string(), "true".to_string());
        env_vars.insert("GOTRUE_LOG_LEVEL".to_string(), "debug".to_string());

        Self {
            env_vars,
            tag: TAG.to_string(),
        }
    }
}

/// Implementation of the Image trait for Auth container
impl Image for Auth {
    /// Returns the name of the Docker image
    fn name(&self) -> &str {
        NAME
    }

    /// Returns the tag of the Docker image
    fn tag(&self) -> &str {
        &self.tag
    }

    /// Specifies the conditions that indicate when the container is ready
    /// Waits for the API to start listening on the configured port
    fn ready_conditions(&self) -> Vec<WaitFor> {
        vec![WaitFor::message_on_stderr("API started")]
    }

    /// Returns the ports to expose from the container
    fn expose_ports(&self) -> &[ContainerPort] {
        &[ContainerPort::Tcp(AUTH_PORT)]
    }

    /// Returns the environment variables to be passed to the container
    fn env_vars(
        &self,
    ) -> impl IntoIterator<Item = (impl Into<Cow<'_, str>>, impl Into<Cow<'_, str>>)> {
        &self.env_vars
    }

    /// Executes commands after the container starts
    ///
    /// # Arguments
    /// * `cs` - Container state
    ///
    /// # Returns
    /// * `Result<Vec<ExecCommand>, TestcontainersError>` - Commands to execute
    #[allow(unused_variables)]
    fn exec_after_start(
        &self,
        cs: ContainerState,
    ) -> Result<Vec<ExecCommand>, TestcontainersError> {
        Ok(vec![])
    }
}

/// Unit tests for Auth container configuration
#[cfg(test)]
mod tests {
    use super::*;
    use testcontainers_modules::testcontainers::Image;

    #[test]
    fn test_default_configuration() {
        let auth = Auth::default();

        // Verify essential env vars are set
        assert_eq!(
            auth.env_vars.get("GOTRUE_DB_DRIVER"),
            Some(&"postgres".to_string())
        );
        assert_eq!(auth.env_vars.get("DB_NAMESPACE"), Some(&"auth".to_string()));
        assert_eq!(auth.env_vars.get("PORT"), Some(&"9999".to_string()));
        assert_eq!(
            auth.env_vars.get("GOTRUE_API_HOST"),
            Some(&"0.0.0.0".to_string())
        );

        // Verify test conveniences are enabled
        assert_eq!(
            auth.env_vars.get("GOTRUE_MAILER_AUTOCONFIRM"),
            Some(&"true".to_string())
        );
        assert_eq!(
            auth.env_vars.get("GOTRUE_SMS_AUTOCONFIRM"),
            Some(&"true".to_string())
        );
    }

    #[test]
    fn test_name_returns_correct_image() {
        let auth = Auth::default();
        assert_eq!(auth.name(), "supabase/gotrue");
    }

    #[test]
    fn test_tag_returns_correct_version() {
        let auth = Auth::default();
        assert_eq!(auth.tag(), "v2.183.0");
    }

    #[test]
    fn test_builder_method_chaining() {
        let auth = Auth::default()
            .with_db_url("postgres://user:pass@localhost:5432/db")
            .with_jwt_secret("my-secret-key")
            .with_jwt_expiry(7200)
            .with_site_url("http://example.com")
            .with_signup_disabled(true)
            .with_anonymous_users(false)
            .with_log_level("info");

        assert_eq!(
            auth.env_vars.get("DATABASE_URL"),
            Some(&"postgres://user:pass@localhost:5432/db".to_string())
        );
        assert_eq!(
            auth.env_vars.get("GOTRUE_JWT_SECRET"),
            Some(&"my-secret-key".to_string())
        );
        assert_eq!(
            auth.env_vars.get("GOTRUE_JWT_EXP"),
            Some(&"7200".to_string())
        );
        assert_eq!(
            auth.env_vars.get("GOTRUE_SITE_URL"),
            Some(&"http://example.com".to_string())
        );
        assert_eq!(
            auth.env_vars.get("GOTRUE_DISABLE_SIGNUP"),
            Some(&"true".to_string())
        );
        assert_eq!(
            auth.env_vars.get("GOTRUE_EXTERNAL_ANONYMOUS_USERS_ENABLED"),
            Some(&"false".to_string())
        );
        assert_eq!(
            auth.env_vars.get("GOTRUE_LOG_LEVEL"),
            Some(&"info".to_string())
        );
    }

    #[test]
    fn test_with_tag_overrides_default() {
        let auth = Auth::default().with_tag("v2.100.0");
        assert_eq!(auth.tag(), "v2.100.0");
    }

    #[test]
    fn test_with_env_adds_custom_variable() {
        let auth = Auth::default()
            .with_env("CUSTOM_VAR", "custom_value")
            .with_env("ANOTHER_VAR", "another_value");

        assert_eq!(
            auth.env_vars.get("CUSTOM_VAR"),
            Some(&"custom_value".to_string())
        );
        assert_eq!(
            auth.env_vars.get("ANOTHER_VAR"),
            Some(&"another_value".to_string())
        );
    }

    #[test]
    fn test_new_sets_database_url() {
        let auth = Auth::new("postgres://test:test@localhost:5432/testdb");
        assert_eq!(
            auth.env_vars.get("DATABASE_URL"),
            Some(&"postgres://test:test@localhost:5432/testdb".to_string())
        );
    }

    #[test]
    fn test_auth_port_constant() {
        assert_eq!(AUTH_PORT, 9999);
    }
}
