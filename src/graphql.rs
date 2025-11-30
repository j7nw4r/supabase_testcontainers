/*! Supabase GraphQL (pg_graphql) container management module.

This module provides a testcontainer implementation for [pg_graphql](https://supabase.github.io/pg_graphql/),
a PostgreSQL extension that provides GraphQL support directly in the database.

# Architecture

pg_graphql is a PostgreSQL extension, not a standalone server. This module provides
a PostgreSQL container with pg_graphql pre-installed via the `supabase/postgres` image.
To query GraphQL over HTTP, you'll need to use PostgREST alongside this container.

The GraphQL workflow:
1. Start this PostgreSQL container (pg_graphql enabled)
2. Create your database schema
3. Start PostgREST pointing to this database
4. Query GraphQL via PostgREST's RPC endpoint

# Example

```rust,no_run
use supabase_testcontainers_modules::{GraphQL, GRAPHQL_PORT, PostgREST, POSTGREST_PORT, DOCKER_INTERNAL_HOST};
use testcontainers::runners::AsyncRunner;

#[tokio::main]
async fn main() -> anyhow::Result<()> {
    // 1. Start PostgreSQL with pg_graphql extension
    let graphql_db = GraphQL::default()
        .with_password("secret")
        .start()
        .await?;

    let db_port = graphql_db.get_host_port_ipv4(GRAPHQL_PORT).await?;

    // 2. Start PostgREST pointing to the database
    let db_url = format!(
        "postgres://postgres:secret@{}:{}/postgres",
        DOCKER_INTERNAL_HOST, db_port
    );

    let postgrest = PostgREST::default()
        .with_postgres_connection(&db_url)
        .with_db_schemas("public,graphql_public")
        .start()
        .await?;

    let rest_port = postgrest.get_host_port_ipv4(POSTGREST_PORT).await?;

    // 3. Query GraphQL via PostgREST RPC
    // POST http://localhost:{rest_port}/rpc/graphql
    // Body: { "query": "{ __typename }" }

    println!("GraphQL available via PostgREST at http://localhost:{}", rest_port);

    Ok(())
}
```

# Configuration

The [`GraphQL`] struct provides builder methods for PostgreSQL configuration:

- [`GraphQL::with_database`] - Database name (default: "postgres")
- [`GraphQL::with_user`] - PostgreSQL user (default: "postgres")
- [`GraphQL::with_password`] - PostgreSQL password (default: "postgres")

pg_graphql-specific configuration is done via SQL comments on database objects:

```sql
-- Enable name inflection for a schema
COMMENT ON SCHEMA public IS e'@graphql({"inflect_names": true})';

-- Set max rows for a table
COMMENT ON TABLE my_table IS e'@graphql({"max_rows": 100})';
```

See the struct documentation for the full list of options.
*/

use std::borrow::Cow;
use std::collections::BTreeMap;

use testcontainers_modules::testcontainers::core::{
    ContainerPort, ContainerState, ExecCommand, WaitFor,
};
use testcontainers_modules::testcontainers::{Image, TestcontainersError};

/// Default image name for Supabase PostgreSQL with pg_graphql
const NAME: &str = "supabase/postgres";
/// Default image tag version
const TAG: &str = "15.8.1.085";
/// Default port for PostgreSQL (pg_graphql is accessed via SQL)
pub const GRAPHQL_PORT: u16 = 5432;

/// Supabase PostgreSQL container with pg_graphql extension for integration testing.
///
/// This struct implements the [`Image`] trait from testcontainers, allowing you to
/// start a PostgreSQL database with the pg_graphql extension pre-installed.
///
/// # Architecture Note
///
/// pg_graphql is a PostgreSQL extension that exposes a `graphql.resolve()` SQL function.
/// To query GraphQL over HTTP, you need to use PostgREST which calls this function
/// via its RPC endpoint.
///
/// # Default Configuration
///
/// The default configuration includes:
/// - Database: "postgres"
/// - User: "postgres"
/// - Password: "postgres"
/// - Port: 5432
/// - pg_graphql extension: pre-installed and ready
///
/// # Example
///
/// ```rust,no_run
/// use supabase_testcontainers_modules::GraphQL;
///
/// let graphql = GraphQL::default()
///     .with_database("mydb")
///     .with_user("myuser")
///     .with_password("mypassword");
/// ```
#[derive(Debug, Clone)]
pub struct GraphQL {
    /// Environment variables to be passed to the container
    env_vars: BTreeMap<String, String>,
    /// Docker image tag version
    tag: String,
}

impl GraphQL {
    /// Creates a new GraphQL instance with default configuration.
    pub fn new() -> Self {
        Self::default()
    }

    /// Creates a new GraphQL instance with custom environment variables.
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

    /// Sets the PostgreSQL database name.
    ///
    /// This database will be created on container startup.
    /// Default is "postgres".
    pub fn with_database(mut self, database: impl Into<String>) -> Self {
        self.env_vars
            .insert("POSTGRES_DB".to_string(), database.into());
        self
    }

    /// Sets the PostgreSQL user.
    ///
    /// This user will be created with superuser privileges.
    /// Default is "postgres".
    pub fn with_user(mut self, user: impl Into<String>) -> Self {
        self.env_vars
            .insert("POSTGRES_USER".to_string(), user.into());
        self
    }

    /// Sets the PostgreSQL password.
    ///
    /// Default is "postgres".
    pub fn with_password(mut self, password: impl Into<String>) -> Self {
        self.env_vars
            .insert("POSTGRES_PASSWORD".to_string(), password.into());
        self
    }

    /// Sets the PostgreSQL host binding address.
    ///
    /// Default is "0.0.0.0" to allow external connections.
    pub fn with_host(mut self, host: impl Into<String>) -> Self {
        self.env_vars
            .insert("POSTGRES_HOST".to_string(), host.into());
        self
    }

    /// Sets the PostgreSQL port.
    ///
    /// Default is 5432.
    pub fn with_port(mut self, port: u16) -> Self {
        self.env_vars
            .insert("POSTGRES_PORT".to_string(), port.to_string());
        self
    }

    /// Sets PostgreSQL initialization mode.
    ///
    /// When set to "true", PostgreSQL will allow connections during initialization.
    pub fn with_host_auth_method(mut self, method: impl Into<String>) -> Self {
        self.env_vars
            .insert("POSTGRES_HOST_AUTH_METHOD".to_string(), method.into());
        self
    }

    /// Sets custom PostgreSQL configuration parameters.
    ///
    /// These are passed to PostgreSQL on startup.
    /// Example: "max_connections=200"
    pub fn with_postgres_args(mut self, args: impl Into<String>) -> Self {
        self.env_vars
            .insert("POSTGRES_INITDB_ARGS".to_string(), args.into());
        self
    }

    /// Enables or disables JWT authentication for pg_graphql.
    ///
    /// When enabled, pg_graphql will validate JWT tokens.
    pub fn with_jwt_secret(mut self, secret: impl Into<String>) -> Self {
        self.env_vars
            .insert("JWT_SECRET".to_string(), secret.into());
        self
    }

    /// Sets a custom Docker image tag/version.
    ///
    /// Use this to pin to a specific Supabase Postgres version.
    pub fn with_tag(mut self, tag: impl Into<String>) -> Self {
        self.tag = tag.into();
        self
    }

    /// Adds a custom environment variable.
    ///
    /// Use this for PostgreSQL configuration options not covered by other methods.
    pub fn with_env(mut self, key: impl Into<String>, value: impl Into<String>) -> Self {
        self.env_vars.insert(key.into(), value.into());
        self
    }

    /// Returns a PostgreSQL connection string for this container.
    ///
    /// Note: This returns the connection string template. You'll need to
    /// replace the host and port with actual values after the container starts.
    pub fn connection_string_template(&self) -> String {
        let user = self
            .env_vars
            .get("POSTGRES_USER")
            .cloned()
            .unwrap_or_else(|| "postgres".to_string());
        let password = self
            .env_vars
            .get("POSTGRES_PASSWORD")
            .cloned()
            .unwrap_or_else(|| "postgres".to_string());
        let database = self
            .env_vars
            .get("POSTGRES_DB")
            .cloned()
            .unwrap_or_else(|| "postgres".to_string());

        format!(
            "postgres://{}:{}@{{host}}:{{port}}/{}",
            user, password, database
        )
    }
}

impl Default for GraphQL {
    fn default() -> Self {
        let mut env_vars = BTreeMap::new();

        // PostgreSQL configuration
        // Note: We don't set POSTGRES_HOST as it interferes with the init scripts
        // The container's default configuration handles network binding correctly
        env_vars.insert("POSTGRES_DB".to_string(), "postgres".to_string());
        env_vars.insert("POSTGRES_USER".to_string(), "postgres".to_string());
        env_vars.insert("POSTGRES_PASSWORD".to_string(), "postgres".to_string());

        Self {
            env_vars,
            tag: TAG.to_string(),
        }
    }
}

impl Image for GraphQL {
    fn name(&self) -> &str {
        NAME
    }

    fn tag(&self) -> &str {
        &self.tag
    }

    fn ready_conditions(&self) -> Vec<WaitFor> {
        // The supabase/postgres image logs "database system is ready" when PostgreSQL
        // is ready to accept connections. The init scripts run in the foreground so
        // by the time we see this message, the database should be fully initialized.
        vec![WaitFor::message_on_stderr(
            "database system is ready to accept connections",
        )]
    }

    fn expose_ports(&self) -> &[ContainerPort] {
        &[ContainerPort::Tcp(GRAPHQL_PORT)]
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
#[cfg(feature = "graphql")]
mod tests {
    use super::*;
    use testcontainers_modules::testcontainers::Image;

    #[test]
    fn test_default_configuration() {
        let graphql = GraphQL::default();

        // Verify essential env vars are set
        assert_eq!(
            graphql.env_vars.get("POSTGRES_DB"),
            Some(&"postgres".to_string())
        );
        assert_eq!(
            graphql.env_vars.get("POSTGRES_USER"),
            Some(&"postgres".to_string())
        );
        assert_eq!(
            graphql.env_vars.get("POSTGRES_PASSWORD"),
            Some(&"postgres".to_string())
        );
        // Note: POSTGRES_HOST and POSTGRES_PORT are not set by default
        // as they can interfere with container init scripts.
        // Use with_host() and with_port() to set custom values if needed.
    }

    #[test]
    fn test_name_returns_correct_image() {
        let graphql = GraphQL::default();
        assert_eq!(graphql.name(), "supabase/postgres");
    }

    #[test]
    fn test_tag_returns_correct_version() {
        let graphql = GraphQL::default();
        assert_eq!(graphql.tag(), TAG);
    }

    #[test]
    fn test_graphql_port_constant() {
        assert_eq!(GRAPHQL_PORT, 5432);
    }

    #[test]
    fn test_with_database() {
        let graphql = GraphQL::default().with_database("mydb");
        assert_eq!(
            graphql.env_vars.get("POSTGRES_DB"),
            Some(&"mydb".to_string())
        );
    }

    #[test]
    fn test_with_user() {
        let graphql = GraphQL::default().with_user("myuser");
        assert_eq!(
            graphql.env_vars.get("POSTGRES_USER"),
            Some(&"myuser".to_string())
        );
    }

    #[test]
    fn test_with_password() {
        let graphql = GraphQL::default().with_password("secret");
        assert_eq!(
            graphql.env_vars.get("POSTGRES_PASSWORD"),
            Some(&"secret".to_string())
        );
    }

    #[test]
    fn test_with_host() {
        let graphql = GraphQL::default().with_host("127.0.0.1");
        assert_eq!(
            graphql.env_vars.get("POSTGRES_HOST"),
            Some(&"127.0.0.1".to_string())
        );
    }

    #[test]
    fn test_with_port() {
        let graphql = GraphQL::default().with_port(5433);
        assert_eq!(
            graphql.env_vars.get("POSTGRES_PORT"),
            Some(&"5433".to_string())
        );
    }

    #[test]
    fn test_with_host_auth_method() {
        let graphql = GraphQL::default().with_host_auth_method("trust");
        assert_eq!(
            graphql.env_vars.get("POSTGRES_HOST_AUTH_METHOD"),
            Some(&"trust".to_string())
        );
    }

    #[test]
    fn test_with_postgres_args() {
        let graphql = GraphQL::default().with_postgres_args("--max_connections=200");
        assert_eq!(
            graphql.env_vars.get("POSTGRES_INITDB_ARGS"),
            Some(&"--max_connections=200".to_string())
        );
    }

    #[test]
    fn test_with_jwt_secret() {
        let graphql = GraphQL::default().with_jwt_secret("my-jwt-secret");
        assert_eq!(
            graphql.env_vars.get("JWT_SECRET"),
            Some(&"my-jwt-secret".to_string())
        );
    }

    #[test]
    fn test_with_tag_overrides_default() {
        let graphql = GraphQL::default().with_tag("15.1.0.100");
        assert_eq!(graphql.tag(), "15.1.0.100");
    }

    #[test]
    fn test_with_env_adds_custom_variable() {
        let graphql = GraphQL::default()
            .with_env("CUSTOM_VAR", "custom_value")
            .with_env("ANOTHER_VAR", "another_value");

        assert_eq!(
            graphql.env_vars.get("CUSTOM_VAR"),
            Some(&"custom_value".to_string())
        );
        assert_eq!(
            graphql.env_vars.get("ANOTHER_VAR"),
            Some(&"another_value".to_string())
        );
    }

    #[test]
    fn test_builder_method_chaining() {
        let graphql = GraphQL::default()
            .with_database("testdb")
            .with_user("testuser")
            .with_password("testpass")
            .with_host("localhost")
            .with_port(5433)
            .with_jwt_secret("my-jwt-secret")
            .with_tag("15.1.0.100");

        assert_eq!(
            graphql.env_vars.get("POSTGRES_DB"),
            Some(&"testdb".to_string())
        );
        assert_eq!(
            graphql.env_vars.get("POSTGRES_USER"),
            Some(&"testuser".to_string())
        );
        assert_eq!(
            graphql.env_vars.get("POSTGRES_PASSWORD"),
            Some(&"testpass".to_string())
        );
        assert_eq!(
            graphql.env_vars.get("POSTGRES_HOST"),
            Some(&"localhost".to_string())
        );
        assert_eq!(
            graphql.env_vars.get("POSTGRES_PORT"),
            Some(&"5433".to_string())
        );
        assert_eq!(
            graphql.env_vars.get("JWT_SECRET"),
            Some(&"my-jwt-secret".to_string())
        );
        assert_eq!(graphql.tag(), "15.1.0.100");
    }

    #[test]
    fn test_new_creates_default_instance() {
        let graphql = GraphQL::new();
        assert_eq!(graphql.name(), NAME);
        assert_eq!(graphql.tag(), TAG);
    }

    #[test]
    fn test_new_with_env() {
        let mut envs = BTreeMap::new();
        envs.insert("CUSTOM_KEY", "custom_value");
        envs.insert("POSTGRES_DB", "customdb");

        let graphql = GraphQL::new_with_env(envs);
        assert_eq!(
            graphql.env_vars.get("CUSTOM_KEY"),
            Some(&"custom_value".to_string())
        );
        // Custom db should override default
        assert_eq!(
            graphql.env_vars.get("POSTGRES_DB"),
            Some(&"customdb".to_string())
        );
    }

    #[test]
    fn test_expose_ports() {
        let graphql = GraphQL::default();
        let ports = graphql.expose_ports();
        assert_eq!(ports.len(), 1);
        assert_eq!(ports[0], ContainerPort::Tcp(5432));
    }

    #[test]
    fn test_ready_conditions() {
        let graphql = GraphQL::default();
        let conditions = graphql.ready_conditions();
        assert_eq!(conditions.len(), 1);
    }

    #[test]
    fn test_connection_string_template() {
        let graphql = GraphQL::default()
            .with_database("mydb")
            .with_user("myuser")
            .with_password("mypass");

        let template = graphql.connection_string_template();
        assert_eq!(template, "postgres://myuser:mypass@{host}:{port}/mydb");
    }

    #[test]
    fn test_connection_string_template_defaults() {
        let graphql = GraphQL::default();
        let template = graphql.connection_string_template();
        assert_eq!(
            template,
            "postgres://postgres:postgres@{host}:{port}/postgres"
        );
    }
}
