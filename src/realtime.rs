/*! Supabase Realtime container management module.

This module provides a testcontainer implementation for [Supabase Realtime](https://github.com/supabase/realtime),
enabling integration testing with real-time PostgreSQL change data capture and WebSocket broadcasting.

# Features

- Full configuration via fluent builder API
- PostgreSQL logical replication support (CDC)
- JWT authentication for secure WebSocket connections
- Configurable replication slot management
- Multi-tenant support

# Example

```rust,no_run
use supabase_testcontainers_modules::{Realtime, REALTIME_PORT, DOCKER_INTERNAL_HOST};
use testcontainers::runners::AsyncRunner;
use testcontainers_modules::postgres::Postgres;

#[tokio::main]
async fn main() -> anyhow::Result<()> {
    // 1. Start PostgreSQL with logical replication enabled
    let postgres = Postgres::default().start().await?;
    let pg_port = postgres.get_host_port_ipv4(5432).await?;

    // 2. Configure connection for Realtime
    let db_url = format!(
        "postgres://postgres:postgres@{}:{}/postgres",
        DOCKER_INTERNAL_HOST, pg_port
    );

    // 3. Start Realtime with PostgreSQL connection
    let realtime = Realtime::default()
        .with_postgres_connection(&db_url)
        .with_jwt_secret("super-secret-jwt-token-with-at-least-32-characters")
        .start()
        .await?;

    let port = realtime.get_host_port_ipv4(REALTIME_PORT).await?;
    println!("Realtime running at ws://localhost:{}", port);

    Ok(())
}
```

# Configuration

The [`Realtime`] struct provides builder methods for common configuration options:

- [`Realtime::with_postgres_connection`] - PostgreSQL connection string (DB_URL)
- [`Realtime::with_db_host`], [`Realtime::with_db_port`], etc. - Individual database config
- [`Realtime::with_jwt_secret`] - JWT signing secret
- [`Realtime::with_slot_name`] - Replication slot name
- [`Realtime::with_temporary_slot`] - Use temporary replication slot
- [`Realtime::with_secure_channels`] - Enable secure WebSocket channels
- [`Realtime::with_region`] - AWS region for multi-region deployments
- [`Realtime::with_tenant_id`] - Tenant identifier for multi-tenant mode

See the struct documentation for the full list of options.
*/

use std::borrow::Cow;
use std::collections::BTreeMap;

use testcontainers_modules::testcontainers::core::{
    ContainerPort, ContainerState, ExecCommand, WaitFor,
};
use testcontainers_modules::testcontainers::{Image, TestcontainersError};

/// Default image name for Supabase Realtime
const NAME: &str = "supabase/realtime";
/// Default image tag version
const TAG: &str = "v2.33.58";
/// Default port for Supabase Realtime WebSocket server
pub const REALTIME_PORT: u16 = 4000;

/// Supabase Realtime container for integration testing.
///
/// This struct implements the [`Image`] trait from testcontainers, allowing you to
/// start a fully configured Supabase Realtime service for testing real-time
/// database changes and WebSocket broadcasting.
///
/// # Default Configuration
///
/// The default configuration includes:
/// - Port set to 4000
/// - Replication slot name: "realtime_rls"
/// - Temporary slot enabled (recommended for testing)
/// - Secure channels enabled
/// - Region set to "local"
/// - Tenant ID set to "realtime-dev"
///
/// # Example
///
/// ```rust,no_run
/// use supabase_testcontainers_modules::Realtime;
///
/// let realtime = Realtime::default()
///     .with_postgres_connection("postgres://user:pass@host:5432/db")
///     .with_jwt_secret("super-secret-jwt-token-with-at-least-32-characters")
///     .with_slot_name("my_custom_slot")
///     .with_temporary_slot(true);
/// ```
#[derive(Debug, Clone)]
pub struct Realtime {
    /// Environment variables to be passed to the container
    env_vars: BTreeMap<String, String>,
    /// Docker image tag version
    tag: String,
}

impl Realtime {
    /// Creates a new Realtime instance with default configuration.
    pub fn new() -> Self {
        Self::default()
    }

    /// Creates a new Realtime instance with custom environment variables.
    ///
    /// Variables provided here will be merged with the defaults,
    /// with custom values taking precedence.
    pub fn new_with_env(envs: BTreeMap<&str, &str>) -> Self {
        let mut instance = Self::default();
        for (key, val) in envs {
            instance.env_vars.insert(key.to_string(), val.to_string());
        }
        instance
    }

    /// Sets the PostgreSQL connection string (DB_URL).
    ///
    /// This is the primary way to configure the database connection.
    /// Format: `postgres://user:password@host:port/database`
    pub fn with_postgres_connection(mut self, connection_string: impl Into<String>) -> Self {
        self.env_vars
            .insert("DB_URL".to_string(), connection_string.into());
        self
    }

    /// Sets the PostgreSQL host.
    ///
    /// Use this for individual database configuration instead of DB_URL.
    pub fn with_db_host(mut self, host: impl Into<String>) -> Self {
        self.env_vars.insert("DB_HOST".to_string(), host.into());
        self
    }

    /// Sets the PostgreSQL port.
    ///
    /// Default is 5432.
    pub fn with_db_port(mut self, port: u16) -> Self {
        self.env_vars
            .insert("DB_PORT".to_string(), port.to_string());
        self
    }

    /// Sets the PostgreSQL database name.
    pub fn with_db_name(mut self, name: impl Into<String>) -> Self {
        self.env_vars.insert("DB_NAME".to_string(), name.into());
        self
    }

    /// Sets the PostgreSQL user.
    pub fn with_db_user(mut self, user: impl Into<String>) -> Self {
        self.env_vars.insert("DB_USER".to_string(), user.into());
        self
    }

    /// Sets the PostgreSQL password.
    pub fn with_db_password(mut self, password: impl Into<String>) -> Self {
        self.env_vars
            .insert("DB_PASSWORD".to_string(), password.into());
        self
    }

    /// Enables or disables SSL for the database connection.
    pub fn with_db_ssl(mut self, enabled: bool) -> Self {
        self.env_vars
            .insert("DB_SSL".to_string(), enabled.to_string());
        self
    }

    /// Sets a query to execute after connecting to the database.
    pub fn with_db_after_connect_query(mut self, query: impl Into<String>) -> Self {
        self.env_vars
            .insert("DB_AFTER_CONNECT_QUERY".to_string(), query.into());
        self
    }

    /// Sets the JWT secret for token validation.
    ///
    /// This secret is used to verify JWT tokens from clients
    /// connecting to the Realtime WebSocket server.
    /// Should be at least 32 characters.
    pub fn with_jwt_secret(mut self, secret: impl Into<String>) -> Self {
        self.env_vars
            .insert("JWT_SECRET".to_string(), secret.into());
        self
    }

    /// Sets the API JWT secret.
    ///
    /// Used for internal API authentication.
    pub fn with_api_jwt_secret(mut self, secret: impl Into<String>) -> Self {
        self.env_vars
            .insert("API_JWT_SECRET".to_string(), secret.into());
        self
    }

    /// Sets the Phoenix secret key base.
    ///
    /// Used by Phoenix framework for signing and encryption.
    /// Should be at least 64 characters.
    pub fn with_secret_key_base(mut self, secret: impl Into<String>) -> Self {
        self.env_vars
            .insert("SECRET_KEY_BASE".to_string(), secret.into());
        self
    }

    /// Sets the replication slot name.
    ///
    /// PostgreSQL replication slots track WAL changes for logical replication.
    /// Default is "realtime_rls".
    pub fn with_slot_name(mut self, name: impl Into<String>) -> Self {
        self.env_vars.insert("SLOT_NAME".to_string(), name.into());
        self
    }

    /// Enables or disables temporary replication slots.
    ///
    /// When true, the replication slot is automatically dropped when
    /// the connection ends. Recommended for testing to avoid slot accumulation.
    /// Default is true.
    pub fn with_temporary_slot(mut self, temporary: bool) -> Self {
        self.env_vars
            .insert("TEMPORARY_SLOT".to_string(), temporary.to_string());
        self
    }

    /// Sets the maximum number of replication slot records.
    pub fn with_max_record_bytes(mut self, bytes: u64) -> Self {
        self.env_vars
            .insert("MAX_RECORD_BYTES".to_string(), bytes.to_string());
        self
    }

    /// Enables or disables secure WebSocket channels.
    ///
    /// When enabled, all channel subscriptions require valid JWT authentication.
    /// Default is true.
    pub fn with_secure_channels(mut self, secure: bool) -> Self {
        self.env_vars
            .insert("SECURE_CHANNELS".to_string(), secure.to_string());
        self
    }

    /// Sets the AWS region.
    ///
    /// Used for multi-region deployments and S3 access.
    /// Default is "local".
    pub fn with_region(mut self, region: impl Into<String>) -> Self {
        self.env_vars.insert("REGION".to_string(), region.into());
        self
    }

    /// Sets the tenant identifier.
    ///
    /// Used for multi-tenant deployments to isolate data.
    /// Default is "realtime-dev".
    pub fn with_tenant_id(mut self, tenant_id: impl Into<String>) -> Self {
        self.env_vars
            .insert("TENANT_ID".to_string(), tenant_id.into());
        self
    }

    /// Sets Erlang runtime flags.
    ///
    /// Used to configure the Erlang VM that runs the Realtime server.
    pub fn with_erl_aflags(mut self, flags: impl Into<String>) -> Self {
        self.env_vars.insert("ERL_AFLAGS".to_string(), flags.into());
        self
    }

    /// Sets the DNS cluster nodes.
    ///
    /// Used for clustering Realtime servers in distributed deployments.
    pub fn with_dns_nodes(mut self, nodes: impl Into<String>) -> Self {
        self.env_vars.insert("DNS_NODES".to_string(), nodes.into());
        self
    }

    /// Enables or disables Tailscale networking.
    pub fn with_enable_tailscale(mut self, enabled: bool) -> Self {
        self.env_vars
            .insert("ENABLE_TAILSCALE".to_string(), enabled.to_string());
        self
    }

    /// Sets the server port.
    ///
    /// Default is 4000.
    pub fn with_port(mut self, port: u16) -> Self {
        self.env_vars.insert("PORT".to_string(), port.to_string());
        self
    }

    /// Sets a custom Docker image tag/version.
    pub fn with_tag(mut self, tag: impl Into<String>) -> Self {
        self.tag = tag.into();
        self
    }

    /// Adds a custom environment variable.
    ///
    /// Use this for Realtime configuration options not covered by other methods.
    pub fn with_env(mut self, key: impl Into<String>, value: impl Into<String>) -> Self {
        self.env_vars.insert(key.into(), value.into());
        self
    }
}

impl Default for Realtime {
    fn default() -> Self {
        let mut env_vars = BTreeMap::new();

        // Server configuration
        env_vars.insert("PORT".to_string(), REALTIME_PORT.to_string());

        // Application name - required by Phoenix runtime config
        env_vars.insert("APP_NAME".to_string(), "realtime".to_string());

        // Replication configuration
        env_vars.insert("SLOT_NAME".to_string(), "realtime_rls".to_string());
        env_vars.insert("TEMPORARY_SLOT".to_string(), "true".to_string());

        // Security configuration
        env_vars.insert("SECURE_CHANNELS".to_string(), "true".to_string());

        // Region and tenant configuration
        env_vars.insert("REGION".to_string(), "local".to_string());
        env_vars.insert("TENANT_ID".to_string(), "realtime-dev".to_string());

        // Erlang configuration for proper networking
        env_vars.insert("ERL_AFLAGS".to_string(), "-proto_dist inet_tcp".to_string());

        // Disable features not needed for testing
        env_vars.insert("ENABLE_TAILSCALE".to_string(), "false".to_string());

        // Database defaults
        env_vars.insert("DB_PORT".to_string(), "5432".to_string());
        env_vars.insert("DB_SSL".to_string(), "false".to_string());

        // Resource limits - required to avoid startup script error
        env_vars.insert("RLIMIT_NOFILE".to_string(), "10000".to_string());

        Self {
            env_vars,
            tag: TAG.to_string(),
        }
    }
}

impl Image for Realtime {
    fn name(&self) -> &str {
        NAME
    }

    fn tag(&self) -> &str {
        &self.tag
    }

    fn ready_conditions(&self) -> Vec<WaitFor> {
        // Realtime logs "Realtime has started" when the Phoenix endpoint is ready
        vec![WaitFor::message_on_stdout("Realtime has started")]
    }

    fn expose_ports(&self) -> &[ContainerPort] {
        &[ContainerPort::Tcp(REALTIME_PORT)]
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
#[cfg(feature = "realtime")]
mod tests {
    use super::*;
    use testcontainers_modules::testcontainers::Image;

    #[test]
    fn test_default_configuration() {
        let realtime = Realtime::default();

        // Verify essential env vars are set
        assert_eq!(realtime.env_vars.get("PORT"), Some(&"4000".to_string()));
        assert_eq!(
            realtime.env_vars.get("APP_NAME"),
            Some(&"realtime".to_string())
        );
        assert_eq!(
            realtime.env_vars.get("SLOT_NAME"),
            Some(&"realtime_rls".to_string())
        );
        assert_eq!(
            realtime.env_vars.get("TEMPORARY_SLOT"),
            Some(&"true".to_string())
        );
        assert_eq!(
            realtime.env_vars.get("SECURE_CHANNELS"),
            Some(&"true".to_string())
        );
        assert_eq!(realtime.env_vars.get("REGION"), Some(&"local".to_string()));
        assert_eq!(
            realtime.env_vars.get("TENANT_ID"),
            Some(&"realtime-dev".to_string())
        );
        assert_eq!(
            realtime.env_vars.get("ERL_AFLAGS"),
            Some(&"-proto_dist inet_tcp".to_string())
        );
        assert_eq!(
            realtime.env_vars.get("ENABLE_TAILSCALE"),
            Some(&"false".to_string())
        );
        assert_eq!(realtime.env_vars.get("DB_PORT"), Some(&"5432".to_string()));
        assert_eq!(realtime.env_vars.get("DB_SSL"), Some(&"false".to_string()));
        assert_eq!(
            realtime.env_vars.get("RLIMIT_NOFILE"),
            Some(&"10000".to_string())
        );
    }

    #[test]
    fn test_name_returns_correct_image() {
        let realtime = Realtime::default();
        assert_eq!(realtime.name(), "supabase/realtime");
    }

    #[test]
    fn test_tag_returns_correct_version() {
        let realtime = Realtime::default();
        assert_eq!(realtime.tag(), TAG);
    }

    #[test]
    fn test_realtime_port_constant() {
        assert_eq!(REALTIME_PORT, 4000);
    }

    #[test]
    fn test_with_postgres_connection() {
        let realtime =
            Realtime::default().with_postgres_connection("postgres://user:pass@localhost:5432/db");
        assert_eq!(
            realtime.env_vars.get("DB_URL"),
            Some(&"postgres://user:pass@localhost:5432/db".to_string())
        );
    }

    #[test]
    fn test_with_db_host() {
        let realtime = Realtime::default().with_db_host("myhost.example.com");
        assert_eq!(
            realtime.env_vars.get("DB_HOST"),
            Some(&"myhost.example.com".to_string())
        );
    }

    #[test]
    fn test_with_db_port() {
        let realtime = Realtime::default().with_db_port(5433);
        assert_eq!(realtime.env_vars.get("DB_PORT"), Some(&"5433".to_string()));
    }

    #[test]
    fn test_with_db_name() {
        let realtime = Realtime::default().with_db_name("mydb");
        assert_eq!(realtime.env_vars.get("DB_NAME"), Some(&"mydb".to_string()));
    }

    #[test]
    fn test_with_db_user() {
        let realtime = Realtime::default().with_db_user("myuser");
        assert_eq!(
            realtime.env_vars.get("DB_USER"),
            Some(&"myuser".to_string())
        );
    }

    #[test]
    fn test_with_db_password() {
        let realtime = Realtime::default().with_db_password("mypassword");
        assert_eq!(
            realtime.env_vars.get("DB_PASSWORD"),
            Some(&"mypassword".to_string())
        );
    }

    #[test]
    fn test_with_db_ssl() {
        let realtime = Realtime::default().with_db_ssl(true);
        assert_eq!(realtime.env_vars.get("DB_SSL"), Some(&"true".to_string()));
    }

    #[test]
    fn test_with_db_after_connect_query() {
        let realtime = Realtime::default().with_db_after_connect_query("SET timezone TO 'UTC'");
        assert_eq!(
            realtime.env_vars.get("DB_AFTER_CONNECT_QUERY"),
            Some(&"SET timezone TO 'UTC'".to_string())
        );
    }

    #[test]
    fn test_with_jwt_secret() {
        let realtime = Realtime::default()
            .with_jwt_secret("super-secret-jwt-token-with-at-least-32-characters");
        assert_eq!(
            realtime.env_vars.get("JWT_SECRET"),
            Some(&"super-secret-jwt-token-with-at-least-32-characters".to_string())
        );
    }

    #[test]
    fn test_with_api_jwt_secret() {
        let realtime = Realtime::default().with_api_jwt_secret("api-secret-key");
        assert_eq!(
            realtime.env_vars.get("API_JWT_SECRET"),
            Some(&"api-secret-key".to_string())
        );
    }

    #[test]
    fn test_with_secret_key_base() {
        let realtime = Realtime::default().with_secret_key_base("phoenix-secret-key-base");
        assert_eq!(
            realtime.env_vars.get("SECRET_KEY_BASE"),
            Some(&"phoenix-secret-key-base".to_string())
        );
    }

    #[test]
    fn test_with_slot_name() {
        let realtime = Realtime::default().with_slot_name("my_custom_slot");
        assert_eq!(
            realtime.env_vars.get("SLOT_NAME"),
            Some(&"my_custom_slot".to_string())
        );
    }

    #[test]
    fn test_with_temporary_slot() {
        let realtime = Realtime::default().with_temporary_slot(false);
        assert_eq!(
            realtime.env_vars.get("TEMPORARY_SLOT"),
            Some(&"false".to_string())
        );
    }

    #[test]
    fn test_with_max_record_bytes() {
        let realtime = Realtime::default().with_max_record_bytes(1048576);
        assert_eq!(
            realtime.env_vars.get("MAX_RECORD_BYTES"),
            Some(&"1048576".to_string())
        );
    }

    #[test]
    fn test_with_secure_channels() {
        let realtime = Realtime::default().with_secure_channels(false);
        assert_eq!(
            realtime.env_vars.get("SECURE_CHANNELS"),
            Some(&"false".to_string())
        );
    }

    #[test]
    fn test_with_region() {
        let realtime = Realtime::default().with_region("us-east-1");
        assert_eq!(
            realtime.env_vars.get("REGION"),
            Some(&"us-east-1".to_string())
        );
    }

    #[test]
    fn test_with_tenant_id() {
        let realtime = Realtime::default().with_tenant_id("my-tenant");
        assert_eq!(
            realtime.env_vars.get("TENANT_ID"),
            Some(&"my-tenant".to_string())
        );
    }

    #[test]
    fn test_with_erl_aflags() {
        let realtime =
            Realtime::default().with_erl_aflags("-kernel inet_dist_use_interface {0,0,0,0}");
        assert_eq!(
            realtime.env_vars.get("ERL_AFLAGS"),
            Some(&"-kernel inet_dist_use_interface {0,0,0,0}".to_string())
        );
    }

    #[test]
    fn test_with_dns_nodes() {
        let realtime = Realtime::default().with_dns_nodes("node1.example.com,node2.example.com");
        assert_eq!(
            realtime.env_vars.get("DNS_NODES"),
            Some(&"node1.example.com,node2.example.com".to_string())
        );
    }

    #[test]
    fn test_with_enable_tailscale() {
        let realtime = Realtime::default().with_enable_tailscale(true);
        assert_eq!(
            realtime.env_vars.get("ENABLE_TAILSCALE"),
            Some(&"true".to_string())
        );
    }

    #[test]
    fn test_with_port() {
        let realtime = Realtime::default().with_port(8080);
        assert_eq!(realtime.env_vars.get("PORT"), Some(&"8080".to_string()));
    }

    #[test]
    fn test_with_tag_overrides_default() {
        let realtime = Realtime::default().with_tag("v2.0.0");
        assert_eq!(realtime.tag(), "v2.0.0");
    }

    #[test]
    fn test_with_env_adds_custom_variable() {
        let realtime = Realtime::default()
            .with_env("CUSTOM_VAR", "custom_value")
            .with_env("ANOTHER_VAR", "another_value");

        assert_eq!(
            realtime.env_vars.get("CUSTOM_VAR"),
            Some(&"custom_value".to_string())
        );
        assert_eq!(
            realtime.env_vars.get("ANOTHER_VAR"),
            Some(&"another_value".to_string())
        );
    }

    #[test]
    fn test_builder_method_chaining() {
        let realtime = Realtime::default()
            .with_postgres_connection("postgres://user:pass@localhost:5432/db")
            .with_jwt_secret("my-jwt-secret")
            .with_slot_name("custom_slot")
            .with_temporary_slot(true)
            .with_secure_channels(false)
            .with_region("eu-west-1")
            .with_tenant_id("my-tenant")
            .with_tag("v3.0.0");

        assert_eq!(
            realtime.env_vars.get("DB_URL"),
            Some(&"postgres://user:pass@localhost:5432/db".to_string())
        );
        assert_eq!(
            realtime.env_vars.get("JWT_SECRET"),
            Some(&"my-jwt-secret".to_string())
        );
        assert_eq!(
            realtime.env_vars.get("SLOT_NAME"),
            Some(&"custom_slot".to_string())
        );
        assert_eq!(
            realtime.env_vars.get("TEMPORARY_SLOT"),
            Some(&"true".to_string())
        );
        assert_eq!(
            realtime.env_vars.get("SECURE_CHANNELS"),
            Some(&"false".to_string())
        );
        assert_eq!(
            realtime.env_vars.get("REGION"),
            Some(&"eu-west-1".to_string())
        );
        assert_eq!(
            realtime.env_vars.get("TENANT_ID"),
            Some(&"my-tenant".to_string())
        );
        assert_eq!(realtime.tag(), "v3.0.0");
    }

    #[test]
    fn test_new_creates_default_instance() {
        let realtime = Realtime::new();
        assert_eq!(realtime.name(), NAME);
        assert_eq!(realtime.tag(), TAG);
    }

    #[test]
    fn test_new_with_env() {
        let mut envs = BTreeMap::new();
        envs.insert("CUSTOM_KEY", "custom_value");
        envs.insert("PORT", "5000");

        let realtime = Realtime::new_with_env(envs);
        assert_eq!(
            realtime.env_vars.get("CUSTOM_KEY"),
            Some(&"custom_value".to_string())
        );
        // Custom port should override default
        assert_eq!(realtime.env_vars.get("PORT"), Some(&"5000".to_string()));
    }

    #[test]
    fn test_expose_ports() {
        let realtime = Realtime::default();
        let ports = realtime.expose_ports();
        assert_eq!(ports.len(), 1);
        assert_eq!(ports[0], ContainerPort::Tcp(4000));
    }

    #[test]
    fn test_ready_conditions() {
        let realtime = Realtime::default();
        let conditions = realtime.ready_conditions();
        assert_eq!(conditions.len(), 1);
    }

    #[test]
    fn test_individual_db_config() {
        let realtime = Realtime::default()
            .with_db_host("db.example.com")
            .with_db_port(5433)
            .with_db_name("mydb")
            .with_db_user("myuser")
            .with_db_password("secret")
            .with_db_ssl(true);

        assert_eq!(
            realtime.env_vars.get("DB_HOST"),
            Some(&"db.example.com".to_string())
        );
        assert_eq!(realtime.env_vars.get("DB_PORT"), Some(&"5433".to_string()));
        assert_eq!(realtime.env_vars.get("DB_NAME"), Some(&"mydb".to_string()));
        assert_eq!(
            realtime.env_vars.get("DB_USER"),
            Some(&"myuser".to_string())
        );
        assert_eq!(
            realtime.env_vars.get("DB_PASSWORD"),
            Some(&"secret".to_string())
        );
        assert_eq!(realtime.env_vars.get("DB_SSL"), Some(&"true".to_string()));
    }
}
