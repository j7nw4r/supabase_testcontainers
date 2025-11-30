/*! Supabase Analytics (Logflare) container management module.

This module provides a testcontainer implementation for [Supabase Analytics](https://supabase.com/docs/guides/self-hosting/analytics/config),
powered by Logflare - a centralized logging and analytics platform.

# Features

- Full configuration via fluent builder API
- PostgreSQL backend support for log storage
- Single-tenant mode for self-hosted deployments
- Configurable access tokens for API authentication
- Health endpoint for readiness checks

# Architecture

Supabase Analytics is built on Logflare, an Elixir/Phoenix application that:
1. Ingests logs from all Supabase services via Vector
2. Stores analytics data in PostgreSQL (in `_analytics` schema)
3. Provides query APIs for log aggregation and search

# Example

```rust,no_run
use supabase_testcontainers_modules::{Analytics, ANALYTICS_PORT};
use testcontainers::runners::AsyncRunner;
use testcontainers_modules::postgres::Postgres;

#[tokio::main]
async fn main() -> anyhow::Result<()> {
    // 1. Start PostgreSQL
    let postgres = Postgres::default().start().await?;
    let pg_port = postgres.get_host_port_ipv4(5432).await?;

    // 2. Configure connection for Analytics
    let db_url = format!(
        "postgresql://postgres:postgres@host.docker.internal:{}/postgres",
        pg_port
    );

    // 3. Start Analytics with PostgreSQL backend
    let analytics = Analytics::default()
        .with_postgres_backend_url(&db_url)
        .with_public_access_token("your-public-token")
        .with_private_access_token("your-private-token")
        .start()
        .await?;

    let port = analytics.get_host_port_ipv4(ANALYTICS_PORT).await?;
    println!("Analytics running at http://localhost:{}", port);

    Ok(())
}
```

# Configuration

The [`Analytics`] struct provides builder methods for common configuration options:

- [`Analytics::with_postgres_backend_url`] - PostgreSQL connection URL for backend storage
- [`Analytics::with_public_access_token`] - Public API token for ingestion/querying
- [`Analytics::with_private_access_token`] - Private API token for management
- [`Analytics::with_encryption_key`] - Base64 encryption key for sensitive data
- [`Analytics::with_log_level`] - Log verbosity (error, warning, info)

See the struct documentation for the full list of options.
*/

use std::borrow::Cow;
use std::collections::BTreeMap;

use testcontainers_modules::testcontainers::core::{
    ContainerPort, ContainerState, ExecCommand, WaitFor,
};
use testcontainers_modules::testcontainers::{Image, TestcontainersError};

/// Default image name for Supabase Analytics (Logflare)
const NAME: &str = "supabase/logflare";
/// Default image tag version
const TAG: &str = "1.26.13";
/// Default port for Supabase Analytics API
pub const ANALYTICS_PORT: u16 = 4000;

/// Supabase Analytics container for integration testing.
///
/// This struct implements the [`Image`] trait from testcontainers, allowing you to
/// start a fully configured Supabase Analytics (Logflare) service for testing
/// log ingestion and querying.
///
/// # Default Configuration
///
/// The default configuration includes:
/// - HTTP port set to 4000
/// - Single-tenant mode enabled
/// - Supabase mode enabled
/// - Node host set to 127.0.0.1
/// - Database schema set to "_analytics"
/// - Feature flag for multibackend enabled
///
/// # Example
///
/// ```rust,no_run
/// use supabase_testcontainers_modules::Analytics;
///
/// let analytics = Analytics::default()
///     .with_postgres_backend_url("postgresql://user:pass@host:5432/db")
///     .with_postgres_backend_schema("_analytics")
///     .with_public_access_token("your-public-token")
///     .with_private_access_token("your-private-token");
/// ```
#[derive(Debug, Clone)]
pub struct Analytics {
    /// Environment variables to be passed to the container
    env_vars: BTreeMap<String, String>,
    /// Docker image tag version
    tag: String,
}

impl Analytics {
    /// Creates a new Analytics instance
    pub fn new() -> Self {
        Self::default()
    }

    /// Creates a new Analytics instance with custom environment variables
    pub fn new_with_env(envs: BTreeMap<&str, &str>) -> Self {
        let mut instance = Self::default();
        for (key, val) in envs {
            instance.env_vars.insert(key.to_string(), val.to_string());
        }
        instance
    }

    /// Sets the PostgreSQL backend URL for analytics storage
    ///
    /// This is the primary storage location for all analytics data.
    /// Format: `postgresql://user:password@host:port/database`
    pub fn with_postgres_backend_url(mut self, url: impl Into<String>) -> Self {
        self.env_vars
            .insert("POSTGRES_BACKEND_URL".to_string(), url.into());
        self
    }

    /// Sets the PostgreSQL backend schema
    ///
    /// Default is "_analytics". This schema is used to isolate analytics data.
    pub fn with_postgres_backend_schema(mut self, schema: impl Into<String>) -> Self {
        self.env_vars
            .insert("POSTGRES_BACKEND_SCHEMA".to_string(), schema.into());
        self
    }

    /// Sets the database hostname
    pub fn with_db_hostname(mut self, hostname: impl Into<String>) -> Self {
        self.env_vars
            .insert("DB_HOSTNAME".to_string(), hostname.into());
        self
    }

    /// Sets the database port
    pub fn with_db_port(mut self, port: u16) -> Self {
        self.env_vars
            .insert("DB_PORT".to_string(), port.to_string());
        self
    }

    /// Sets the database username
    pub fn with_db_username(mut self, username: impl Into<String>) -> Self {
        self.env_vars
            .insert("DB_USERNAME".to_string(), username.into());
        self
    }

    /// Sets the database password
    pub fn with_db_password(mut self, password: impl Into<String>) -> Self {
        self.env_vars
            .insert("DB_PASSWORD".to_string(), password.into());
        self
    }

    /// Sets the database name
    pub fn with_db_database(mut self, database: impl Into<String>) -> Self {
        self.env_vars
            .insert("DB_DATABASE".to_string(), database.into());
        self
    }

    /// Sets the database schema
    ///
    /// This is an alternative to `with_postgres_backend_schema`.
    pub fn with_db_schema(mut self, schema: impl Into<String>) -> Self {
        self.env_vars.insert("DB_SCHEMA".to_string(), schema.into());
        self
    }

    /// Sets the public access token for API authentication
    ///
    /// This token is used for log ingestion and querying operations.
    pub fn with_public_access_token(mut self, token: impl Into<String>) -> Self {
        self.env_vars
            .insert("LOGFLARE_PUBLIC_ACCESS_TOKEN".to_string(), token.into());
        self
    }

    /// Sets the private access token for management API
    ///
    /// This token is used for administrative operations.
    pub fn with_private_access_token(mut self, token: impl Into<String>) -> Self {
        self.env_vars
            .insert("LOGFLARE_PRIVATE_ACCESS_TOKEN".to_string(), token.into());
        self
    }

    /// Sets the Base64 encryption key for sensitive data at rest
    ///
    /// This key is used to encrypt sensitive database columns.
    pub fn with_encryption_key(mut self, key: impl Into<String>) -> Self {
        self.env_vars
            .insert("LOGFLARE_DB_ENCRYPTION_KEY".to_string(), key.into());
        self
    }

    /// Sets the Logflare node host
    ///
    /// Default is "127.0.0.1". This is used in the Erlang node name.
    pub fn with_node_host(mut self, host: impl Into<String>) -> Self {
        self.env_vars
            .insert("LOGFLARE_NODE_HOST".to_string(), host.into());
        self
    }

    /// Enables or disables single-tenant mode
    ///
    /// Default is `true`. Single-tenant mode seeds a singular user into the database
    /// and disables browser authentication.
    pub fn with_single_tenant(mut self, enabled: bool) -> Self {
        self.env_vars
            .insert("LOGFLARE_SINGLE_TENANT".to_string(), enabled.to_string());
        self
    }

    /// Enables or disables Supabase mode
    ///
    /// Default is `true`. Supabase mode seeds additional resources for usage
    /// with Supabase self-hosted deployments.
    pub fn with_supabase_mode(mut self, enabled: bool) -> Self {
        self.env_vars
            .insert("LOGFLARE_SUPABASE_MODE".to_string(), enabled.to_string());
        self
    }

    /// Sets feature flag overrides
    ///
    /// Format: `flag1=value1,flag2=value2`
    /// Default includes `multibackend=true`.
    pub fn with_feature_flag_override(mut self, flags: impl Into<String>) -> Self {
        self.env_vars
            .insert("LOGFLARE_FEATURE_FLAG_OVERRIDE".to_string(), flags.into());
        self
    }

    /// Sets the log level
    ///
    /// Valid values: "error", "warning", "info"
    pub fn with_log_level(mut self, level: impl Into<String>) -> Self {
        self.env_vars
            .insert("LOGFLARE_LOG_LEVEL".to_string(), level.into());
        self
    }

    /// Sets the Phoenix HTTP port
    ///
    /// Default is 4000.
    pub fn with_http_port(mut self, port: u16) -> Self {
        self.env_vars
            .insert("PHX_HTTP_PORT".to_string(), port.to_string());
        self
    }

    /// Sets a custom Docker image tag/version
    pub fn with_tag(mut self, tag: impl Into<String>) -> Self {
        self.tag = tag.into();
        self
    }

    /// Adds a custom environment variable
    ///
    /// Use this for Logflare configuration options not covered by other methods.
    pub fn with_env(mut self, key: impl Into<String>, value: impl Into<String>) -> Self {
        self.env_vars.insert(key.into(), value.into());
        self
    }
}

impl Default for Analytics {
    fn default() -> Self {
        let mut env_vars = BTreeMap::new();

        // Server configuration
        env_vars.insert("PHX_HTTP_PORT".to_string(), ANALYTICS_PORT.to_string());
        env_vars.insert("LOGFLARE_NODE_HOST".to_string(), "127.0.0.1".to_string());

        // Single-tenant mode (required for self-hosted)
        env_vars.insert("LOGFLARE_SINGLE_TENANT".to_string(), "true".to_string());
        env_vars.insert("LOGFLARE_SUPABASE_MODE".to_string(), "true".to_string());

        // Database schema configuration
        env_vars.insert("DB_SCHEMA".to_string(), "_analytics".to_string());
        env_vars.insert(
            "POSTGRES_BACKEND_SCHEMA".to_string(),
            "_analytics".to_string(),
        );

        // Feature flags
        env_vars.insert(
            "LOGFLARE_FEATURE_FLAG_OVERRIDE".to_string(),
            "multibackend=true".to_string(),
        );

        Self {
            env_vars,
            tag: TAG.to_string(),
        }
    }
}

impl Image for Analytics {
    fn name(&self) -> &str {
        NAME
    }

    fn tag(&self) -> &str {
        &self.tag
    }

    fn ready_conditions(&self) -> Vec<WaitFor> {
        // Logflare/Phoenix logs startup message when server is ready
        vec![WaitFor::message_on_stdout("Starting migration")]
    }

    fn expose_ports(&self) -> &[ContainerPort] {
        &[ContainerPort::Tcp(ANALYTICS_PORT)]
    }

    fn env_vars(
        &self,
    ) -> impl IntoIterator<Item = (impl Into<Cow<'_, str>>, impl Into<Cow<'_, str>>)> {
        &self.env_vars
    }

    #[allow(unused_variables)]
    fn exec_after_start(
        &self,
        cs: ContainerState,
    ) -> Result<Vec<ExecCommand>, TestcontainersError> {
        Ok(vec![])
    }
}

#[cfg(test)]
#[cfg(feature = "analytics")]
mod tests {
    use super::*;
    use testcontainers_modules::testcontainers::Image;

    #[test]
    fn test_default_configuration() {
        let analytics = Analytics::default();

        // Verify essential env vars are set
        assert_eq!(
            analytics.env_vars.get("PHX_HTTP_PORT"),
            Some(&"4000".to_string())
        );
        assert_eq!(
            analytics.env_vars.get("LOGFLARE_NODE_HOST"),
            Some(&"127.0.0.1".to_string())
        );
        assert_eq!(
            analytics.env_vars.get("LOGFLARE_SINGLE_TENANT"),
            Some(&"true".to_string())
        );
        assert_eq!(
            analytics.env_vars.get("LOGFLARE_SUPABASE_MODE"),
            Some(&"true".to_string())
        );
        assert_eq!(
            analytics.env_vars.get("DB_SCHEMA"),
            Some(&"_analytics".to_string())
        );
        assert_eq!(
            analytics.env_vars.get("POSTGRES_BACKEND_SCHEMA"),
            Some(&"_analytics".to_string())
        );
        assert_eq!(
            analytics.env_vars.get("LOGFLARE_FEATURE_FLAG_OVERRIDE"),
            Some(&"multibackend=true".to_string())
        );
    }

    #[test]
    fn test_name_returns_correct_image() {
        let analytics = Analytics::default();
        assert_eq!(analytics.name(), "supabase/logflare");
    }

    #[test]
    fn test_tag_returns_correct_version() {
        let analytics = Analytics::default();
        assert_eq!(analytics.tag(), "1.26.13");
    }

    #[test]
    fn test_analytics_port_constant() {
        assert_eq!(ANALYTICS_PORT, 4000);
    }

    #[test]
    fn test_with_postgres_backend_url() {
        let analytics = Analytics::default()
            .with_postgres_backend_url("postgresql://user:pass@localhost:5432/db");
        assert_eq!(
            analytics.env_vars.get("POSTGRES_BACKEND_URL"),
            Some(&"postgresql://user:pass@localhost:5432/db".to_string())
        );
    }

    #[test]
    fn test_with_postgres_backend_schema() {
        let analytics = Analytics::default().with_postgres_backend_schema("custom_schema");
        assert_eq!(
            analytics.env_vars.get("POSTGRES_BACKEND_SCHEMA"),
            Some(&"custom_schema".to_string())
        );
    }

    #[test]
    fn test_with_db_hostname() {
        let analytics = Analytics::default().with_db_hostname("db.example.com");
        assert_eq!(
            analytics.env_vars.get("DB_HOSTNAME"),
            Some(&"db.example.com".to_string())
        );
    }

    #[test]
    fn test_with_db_port() {
        let analytics = Analytics::default().with_db_port(5433);
        assert_eq!(analytics.env_vars.get("DB_PORT"), Some(&"5433".to_string()));
    }

    #[test]
    fn test_with_db_username() {
        let analytics = Analytics::default().with_db_username("supabase_admin");
        assert_eq!(
            analytics.env_vars.get("DB_USERNAME"),
            Some(&"supabase_admin".to_string())
        );
    }

    #[test]
    fn test_with_db_password() {
        let analytics = Analytics::default().with_db_password("secret123");
        assert_eq!(
            analytics.env_vars.get("DB_PASSWORD"),
            Some(&"secret123".to_string())
        );
    }

    #[test]
    fn test_with_db_database() {
        let analytics = Analytics::default().with_db_database("_supabase");
        assert_eq!(
            analytics.env_vars.get("DB_DATABASE"),
            Some(&"_supabase".to_string())
        );
    }

    #[test]
    fn test_with_db_schema() {
        let analytics = Analytics::default().with_db_schema("my_schema");
        assert_eq!(
            analytics.env_vars.get("DB_SCHEMA"),
            Some(&"my_schema".to_string())
        );
    }

    #[test]
    fn test_with_public_access_token() {
        let analytics = Analytics::default().with_public_access_token("public-token-123");
        assert_eq!(
            analytics.env_vars.get("LOGFLARE_PUBLIC_ACCESS_TOKEN"),
            Some(&"public-token-123".to_string())
        );
    }

    #[test]
    fn test_with_private_access_token() {
        let analytics = Analytics::default().with_private_access_token("private-token-456");
        assert_eq!(
            analytics.env_vars.get("LOGFLARE_PRIVATE_ACCESS_TOKEN"),
            Some(&"private-token-456".to_string())
        );
    }

    #[test]
    fn test_with_encryption_key() {
        let analytics = Analytics::default().with_encryption_key("base64-encoded-key==");
        assert_eq!(
            analytics.env_vars.get("LOGFLARE_DB_ENCRYPTION_KEY"),
            Some(&"base64-encoded-key==".to_string())
        );
    }

    #[test]
    fn test_with_node_host() {
        let analytics = Analytics::default().with_node_host("192.168.1.1");
        assert_eq!(
            analytics.env_vars.get("LOGFLARE_NODE_HOST"),
            Some(&"192.168.1.1".to_string())
        );
    }

    #[test]
    fn test_with_single_tenant() {
        let analytics = Analytics::default().with_single_tenant(false);
        assert_eq!(
            analytics.env_vars.get("LOGFLARE_SINGLE_TENANT"),
            Some(&"false".to_string())
        );
    }

    #[test]
    fn test_with_supabase_mode() {
        let analytics = Analytics::default().with_supabase_mode(false);
        assert_eq!(
            analytics.env_vars.get("LOGFLARE_SUPABASE_MODE"),
            Some(&"false".to_string())
        );
    }

    #[test]
    fn test_with_feature_flag_override() {
        let analytics =
            Analytics::default().with_feature_flag_override("multibackend=true,feature2=false");
        assert_eq!(
            analytics.env_vars.get("LOGFLARE_FEATURE_FLAG_OVERRIDE"),
            Some(&"multibackend=true,feature2=false".to_string())
        );
    }

    #[test]
    fn test_with_log_level() {
        let analytics = Analytics::default().with_log_level("warning");
        assert_eq!(
            analytics.env_vars.get("LOGFLARE_LOG_LEVEL"),
            Some(&"warning".to_string())
        );
    }

    #[test]
    fn test_with_http_port() {
        let analytics = Analytics::default().with_http_port(8080);
        assert_eq!(
            analytics.env_vars.get("PHX_HTTP_PORT"),
            Some(&"8080".to_string())
        );
    }

    #[test]
    fn test_with_tag_overrides_default() {
        let analytics = Analytics::default().with_tag("1.25.0");
        assert_eq!(analytics.tag(), "1.25.0");
    }

    #[test]
    fn test_with_env_adds_custom_variable() {
        let analytics = Analytics::default()
            .with_env("CUSTOM_VAR", "custom_value")
            .with_env("ANOTHER_VAR", "another_value");

        assert_eq!(
            analytics.env_vars.get("CUSTOM_VAR"),
            Some(&"custom_value".to_string())
        );
        assert_eq!(
            analytics.env_vars.get("ANOTHER_VAR"),
            Some(&"another_value".to_string())
        );
    }

    #[test]
    fn test_builder_method_chaining() {
        let analytics = Analytics::default()
            .with_postgres_backend_url("postgresql://user:pass@localhost:5432/db")
            .with_postgres_backend_schema("_analytics")
            .with_public_access_token("public-token")
            .with_private_access_token("private-token")
            .with_encryption_key("base64key==")
            .with_log_level("info")
            .with_tag("1.25.0");

        assert_eq!(
            analytics.env_vars.get("POSTGRES_BACKEND_URL"),
            Some(&"postgresql://user:pass@localhost:5432/db".to_string())
        );
        assert_eq!(
            analytics.env_vars.get("POSTGRES_BACKEND_SCHEMA"),
            Some(&"_analytics".to_string())
        );
        assert_eq!(
            analytics.env_vars.get("LOGFLARE_PUBLIC_ACCESS_TOKEN"),
            Some(&"public-token".to_string())
        );
        assert_eq!(
            analytics.env_vars.get("LOGFLARE_PRIVATE_ACCESS_TOKEN"),
            Some(&"private-token".to_string())
        );
        assert_eq!(
            analytics.env_vars.get("LOGFLARE_DB_ENCRYPTION_KEY"),
            Some(&"base64key==".to_string())
        );
        assert_eq!(
            analytics.env_vars.get("LOGFLARE_LOG_LEVEL"),
            Some(&"info".to_string())
        );
        assert_eq!(analytics.tag(), "1.25.0");
    }

    #[test]
    fn test_new_creates_default_instance() {
        let analytics = Analytics::new();
        assert_eq!(analytics.name(), NAME);
        assert_eq!(analytics.tag(), TAG);
    }

    #[test]
    fn test_expose_ports() {
        let analytics = Analytics::default();
        let ports = analytics.expose_ports();
        assert_eq!(ports.len(), 1);
        assert_eq!(ports[0], ContainerPort::Tcp(4000));
    }

    #[test]
    fn test_ready_conditions() {
        let analytics = Analytics::default();
        let conditions = analytics.ready_conditions();
        assert_eq!(conditions.len(), 1);
    }
}
