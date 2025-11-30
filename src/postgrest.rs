/*! PostgREST container management module.

This module provides a testcontainer implementation for [PostgREST](https://postgrest.org),
enabling integration testing with a real PostgREST API server.

# Features

- Full configuration via fluent builder API
- JWT authentication support
- OpenAPI mode configuration
- Row limiting and pre-request hooks

# Example

```rust,no_run
use supabase_testcontainers_modules::{PostgREST, POSTGREST_PORT, DOCKER_INTERNAL_HOST, LOCAL_HOST};
use testcontainers::runners::AsyncRunner;
use testcontainers_modules::postgres::Postgres;

#[tokio::main]
async fn main() -> anyhow::Result<()> {
    // 1. Start PostgreSQL
    let postgres = Postgres::default().start().await?;
    let pg_port = postgres.get_host_port_ipv4(5432).await?;

    // 2. Configure connection string for PostgREST
    let db_uri = format!(
        "postgres://postgres:postgres@{}:{}/postgres",
        DOCKER_INTERNAL_HOST, pg_port
    );

    // 3. Start PostgREST
    let postgrest = PostgREST::default()
        .with_postgres_connection(&db_uri)
        .with_db_schemas("public,api")
        .with_jwt_secret("super-secret-jwt-token-for-testing")
        .start()
        .await?;

    let port = postgrest.get_host_port_ipv4(POSTGREST_PORT).await?;
    println!("PostgREST running at http://localhost:{}", port);

    Ok(())
}
```

# Configuration

The [`PostgREST`] struct provides builder methods for common configuration options:

- [`PostgREST::with_postgres_connection`] - PostgreSQL connection string
- [`PostgREST::with_db_schemas`] - Exposed database schemas
- [`PostgREST::with_db_anon_role`] - Anonymous role for unauthenticated requests
- [`PostgREST::with_jwt_secret`] - JWT validation secret
- [`PostgREST::with_max_rows`] - Maximum rows per response
- [`PostgREST::with_openapi_mode`] - OpenAPI schema generation mode

See the struct documentation for the full list of options.
*/

use std::borrow::Cow;
use std::collections::BTreeMap;

use testcontainers_modules::testcontainers::core::{
    ContainerPort, ContainerState, ExecCommand, WaitFor,
};
use testcontainers_modules::testcontainers::{Image, TestcontainersError};

/// Default image name for PostgREST
const NAME: &str = "postgrest/postgrest";
/// Default image tag version
const TAG: &str = "v12.2.3";
/// Default port for PostgREST API
pub const POSTGREST_PORT: u16 = 3000;

/// PostgREST container for integration testing.
///
/// This struct implements the [`Image`] trait from testcontainers, allowing you to
/// start a fully configured PostgREST API server for testing database access via REST.
///
/// # Default Configuration
///
/// The default configuration includes:
/// - Schema set to "public"
/// - Anonymous role set to "anon"
/// - Server bound to 0.0.0.0:3000
///
/// # Example
///
/// ```rust,no_run
/// use supabase_testcontainers_modules::PostgREST;
///
/// let postgrest = PostgREST::default()
///     .with_postgres_connection("postgres://user:pass@host:5432/db")
///     .with_jwt_secret("my-secret-key")
///     .with_max_rows(1000);
/// ```
#[derive(Debug, Clone)]
pub struct PostgREST {
    /// Environment variables to be passed to the container
    env_vars: BTreeMap<String, String>,
    /// Docker image tag version
    tag: String,
}

impl PostgREST {
    /// Creates a new PostgREST instance
    pub fn new() -> Self {
        Self::default()
    }

    /// Creates a new PostgREST instance with custom environment variables
    pub fn new_with_env(envs: BTreeMap<&str, &str>) -> Self {
        let mut instance = Self::default();
        for (key, val) in envs {
            instance.env_vars.insert(key.to_string(), val.to_string());
        }
        instance
    }

    /// Sets the PostgreSQL connection string
    pub fn with_postgres_connection(mut self, connection_string: &str) -> Self {
        self.env_vars
            .insert("PGRST_DB_URI".to_string(), connection_string.to_string());
        self
    }

    /// Sets the database schemas (comma-separated for multiple)
    pub fn with_db_schemas(mut self, schemas: &str) -> Self {
        self.env_vars
            .insert("PGRST_DB_SCHEMAS".to_string(), schemas.to_string());
        self
    }

    /// Sets the database anon role
    pub fn with_db_anon_role(mut self, role: &str) -> Self {
        self.env_vars
            .insert("PGRST_DB_ANON_ROLE".to_string(), role.to_string());
        self
    }

    /// Sets the JWT secret for authenticating requests
    ///
    /// When set, PostgREST will validate JWT tokens in the Authorization header
    /// and extract claims for role-based access control.
    pub fn with_jwt_secret(mut self, secret: impl Into<String>) -> Self {
        self.env_vars
            .insert("PGRST_JWT_SECRET".to_string(), secret.into());
        self
    }

    /// Sets the path to the role claim in the JWT payload
    ///
    /// Default is `.role`. Can be a nested path like `.app_metadata.role`
    pub fn with_jwt_role_claim_key(mut self, key: impl Into<String>) -> Self {
        self.env_vars
            .insert("PGRST_JWT_ROLE_CLAIM_KEY".to_string(), key.into());
        self
    }

    /// Sets the OpenAPI mode for schema introspection
    ///
    /// Valid values:
    /// - "follow-privileges": Only show endpoints the user has access to
    /// - "ignore-privileges": Show all endpoints regardless of privileges
    /// - "disabled": Disable OpenAPI output entirely
    pub fn with_openapi_mode(mut self, mode: impl Into<String>) -> Self {
        self.env_vars
            .insert("PGRST_OPENAPI_MODE".to_string(), mode.into());
        self
    }

    /// Sets the maximum number of rows returned by a request
    ///
    /// Limits the number of rows PostgREST will return. Use `None` or 0 for unlimited.
    pub fn with_max_rows(mut self, max_rows: u32) -> Self {
        self.env_vars
            .insert("PGRST_DB_MAX_ROWS".to_string(), max_rows.to_string());
        self
    }

    /// Sets a stored procedure to call before every request
    ///
    /// The function must be in the exposed schemas and will receive the request
    /// method and path as parameters.
    pub fn with_pre_request(mut self, function_name: impl Into<String>) -> Self {
        self.env_vars
            .insert("PGRST_DB_PRE_REQUEST".to_string(), function_name.into());
        self
    }

    /// Sets the log level for PostgREST
    ///
    /// Valid values: "crit", "error", "warn", "info"
    pub fn with_log_level(mut self, level: impl Into<String>) -> Self {
        self.env_vars
            .insert("PGRST_LOG_LEVEL".to_string(), level.into());
        self
    }

    /// Sets a custom Docker image tag/version
    pub fn with_tag(mut self, tag: impl Into<String>) -> Self {
        self.tag = tag.into();
        self
    }

    /// Adds a custom environment variable
    ///
    /// Use this for PostgREST configuration options not covered by other methods.
    pub fn with_env(mut self, key: impl Into<String>, value: impl Into<String>) -> Self {
        self.env_vars.insert(key.into(), value.into());
        self
    }
}

impl Default for PostgREST {
    fn default() -> Self {
        let mut env_vars = BTreeMap::new();
        // Database configuration
        env_vars.insert("PGRST_DB_SCHEMAS".to_string(), "public".to_string());
        env_vars.insert("PGRST_DB_ANON_ROLE".to_string(), "anon".to_string());

        // Server configuration
        env_vars.insert("PGRST_SERVER_PORT".to_string(), POSTGREST_PORT.to_string());
        env_vars.insert("PGRST_SERVER_HOST".to_string(), "0.0.0.0".to_string());

        Self {
            env_vars,
            tag: TAG.to_string(),
        }
    }
}

impl Image for PostgREST {
    fn name(&self) -> &str {
        NAME
    }

    fn tag(&self) -> &str {
        &self.tag
    }

    fn ready_conditions(&self) -> Vec<WaitFor> {
        // PostgREST logs to stderr, not stdout
        vec![WaitFor::message_on_stderr("Listening on port")]
    }

    fn expose_ports(&self) -> &[ContainerPort] {
        &[ContainerPort::Tcp(POSTGREST_PORT)]
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
#[cfg(feature = "postgrest")]
mod tests {
    use super::*;
    use testcontainers_modules::testcontainers::Image;

    #[test]
    fn test_default_configuration() {
        let postgrest = PostgREST::default();

        // Verify essential env vars are set
        assert_eq!(
            postgrest.env_vars.get("PGRST_DB_SCHEMAS"),
            Some(&"public".to_string())
        );
        assert_eq!(
            postgrest.env_vars.get("PGRST_DB_ANON_ROLE"),
            Some(&"anon".to_string())
        );
        assert_eq!(
            postgrest.env_vars.get("PGRST_SERVER_PORT"),
            Some(&"3000".to_string())
        );
        assert_eq!(
            postgrest.env_vars.get("PGRST_SERVER_HOST"),
            Some(&"0.0.0.0".to_string())
        );
    }

    #[test]
    fn test_name_returns_correct_image() {
        let postgrest = PostgREST::default();
        assert_eq!(postgrest.name(), "postgrest/postgrest");
    }

    #[test]
    fn test_tag_returns_correct_version() {
        let postgrest = PostgREST::default();
        assert_eq!(postgrest.tag(), "v12.2.3");
    }

    #[test]
    fn test_postgrest_port_constant() {
        assert_eq!(POSTGREST_PORT, 3000);
    }

    #[test]
    fn test_with_postgres_connection() {
        let postgrest =
            PostgREST::default().with_postgres_connection("postgres://user:pass@localhost:5432/db");
        assert_eq!(
            postgrest.env_vars.get("PGRST_DB_URI"),
            Some(&"postgres://user:pass@localhost:5432/db".to_string())
        );
    }

    #[test]
    fn test_with_db_schemas() {
        let postgrest = PostgREST::default().with_db_schemas("public,api,private");
        assert_eq!(
            postgrest.env_vars.get("PGRST_DB_SCHEMAS"),
            Some(&"public,api,private".to_string())
        );
    }

    #[test]
    fn test_with_db_anon_role() {
        let postgrest = PostgREST::default().with_db_anon_role("web_anon");
        assert_eq!(
            postgrest.env_vars.get("PGRST_DB_ANON_ROLE"),
            Some(&"web_anon".to_string())
        );
    }

    #[test]
    fn test_with_jwt_secret() {
        let postgrest = PostgREST::default().with_jwt_secret("my-super-secret-jwt-key");
        assert_eq!(
            postgrest.env_vars.get("PGRST_JWT_SECRET"),
            Some(&"my-super-secret-jwt-key".to_string())
        );
    }

    #[test]
    fn test_with_jwt_role_claim_key() {
        let postgrest = PostgREST::default().with_jwt_role_claim_key(".app_metadata.role");
        assert_eq!(
            postgrest.env_vars.get("PGRST_JWT_ROLE_CLAIM_KEY"),
            Some(&".app_metadata.role".to_string())
        );
    }

    #[test]
    fn test_with_openapi_mode() {
        let postgrest = PostgREST::default().with_openapi_mode("follow-privileges");
        assert_eq!(
            postgrest.env_vars.get("PGRST_OPENAPI_MODE"),
            Some(&"follow-privileges".to_string())
        );
    }

    #[test]
    fn test_with_max_rows() {
        let postgrest = PostgREST::default().with_max_rows(1000);
        assert_eq!(
            postgrest.env_vars.get("PGRST_DB_MAX_ROWS"),
            Some(&"1000".to_string())
        );
    }

    #[test]
    fn test_with_pre_request() {
        let postgrest = PostgREST::default().with_pre_request("auth.check_request");
        assert_eq!(
            postgrest.env_vars.get("PGRST_DB_PRE_REQUEST"),
            Some(&"auth.check_request".to_string())
        );
    }

    #[test]
    fn test_with_log_level() {
        let postgrest = PostgREST::default().with_log_level("warn");
        assert_eq!(
            postgrest.env_vars.get("PGRST_LOG_LEVEL"),
            Some(&"warn".to_string())
        );
    }

    #[test]
    fn test_with_tag_overrides_default() {
        let postgrest = PostgREST::default().with_tag("v11.0.0");
        assert_eq!(postgrest.tag(), "v11.0.0");
    }

    #[test]
    fn test_with_env_adds_custom_variable() {
        let postgrest = PostgREST::default()
            .with_env("CUSTOM_VAR", "custom_value")
            .with_env("ANOTHER_VAR", "another_value");

        assert_eq!(
            postgrest.env_vars.get("CUSTOM_VAR"),
            Some(&"custom_value".to_string())
        );
        assert_eq!(
            postgrest.env_vars.get("ANOTHER_VAR"),
            Some(&"another_value".to_string())
        );
    }

    #[test]
    fn test_builder_method_chaining() {
        let postgrest = PostgREST::default()
            .with_postgres_connection("postgres://user:pass@localhost:5432/db")
            .with_db_schemas("api")
            .with_db_anon_role("web_anon")
            .with_jwt_secret("secret")
            .with_max_rows(500)
            .with_log_level("info")
            .with_tag("v11.0.0");

        assert_eq!(
            postgrest.env_vars.get("PGRST_DB_URI"),
            Some(&"postgres://user:pass@localhost:5432/db".to_string())
        );
        assert_eq!(
            postgrest.env_vars.get("PGRST_DB_SCHEMAS"),
            Some(&"api".to_string())
        );
        assert_eq!(
            postgrest.env_vars.get("PGRST_DB_ANON_ROLE"),
            Some(&"web_anon".to_string())
        );
        assert_eq!(
            postgrest.env_vars.get("PGRST_JWT_SECRET"),
            Some(&"secret".to_string())
        );
        assert_eq!(
            postgrest.env_vars.get("PGRST_DB_MAX_ROWS"),
            Some(&"500".to_string())
        );
        assert_eq!(
            postgrest.env_vars.get("PGRST_LOG_LEVEL"),
            Some(&"info".to_string())
        );
        assert_eq!(postgrest.tag(), "v11.0.0");
    }

    #[test]
    fn test_new_creates_default_instance() {
        let postgrest = PostgREST::new();
        assert_eq!(postgrest.name(), NAME);
        assert_eq!(postgrest.tag(), TAG);
    }

    #[test]
    fn test_expose_ports() {
        let postgrest = PostgREST::default();
        let ports = postgrest.expose_ports();
        assert_eq!(ports.len(), 1);
        assert_eq!(ports[0], ContainerPort::Tcp(3000));
    }

    #[test]
    fn test_ready_conditions() {
        let postgrest = PostgREST::default();
        let conditions = postgrest.ready_conditions();
        assert_eq!(conditions.len(), 1);
    }
}
