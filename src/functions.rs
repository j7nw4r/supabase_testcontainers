/*! Supabase Edge Functions container management module.

This module provides a testcontainer implementation for [Supabase Edge Functions](https://supabase.com/docs/guides/functions)
(edge-runtime), enabling integration testing with serverless Deno-based functions.

# Features

- Full configuration via fluent builder API
- Deno-based runtime for JavaScript, TypeScript, and WASM
- JWT authentication for secure function invocation
- Direct PostgreSQL database access
- Integration with Supabase services via API keys

# Example

```rust,no_run
use supabase_testcontainers_modules::{Functions, FUNCTIONS_PORT, DOCKER_INTERNAL_HOST};
use testcontainers::runners::AsyncRunner;
use testcontainers_modules::postgres::Postgres;

#[tokio::main]
async fn main() -> anyhow::Result<()> {
    // 1. Start PostgreSQL
    let postgres = Postgres::default().start().await?;
    let pg_port = postgres.get_host_port_ipv4(5432).await?;

    // 2. Configure connection for Edge Functions
    let db_url = format!(
        "postgres://postgres:postgres@{}:{}/postgres",
        DOCKER_INTERNAL_HOST, pg_port
    );

    // 3. Start Edge Functions runtime
    let functions = Functions::default()
        .with_db_url(&db_url)
        .with_jwt_secret("super-secret-jwt-token-with-at-least-32-characters")
        .with_verify_jwt(false) // Disable for testing
        .start()
        .await?;

    let port = functions.get_host_port_ipv4(FUNCTIONS_PORT).await?;
    println!("Edge Functions running at http://localhost:{}", port);

    Ok(())
}
```

# Configuration

The [`Functions`] struct provides builder methods for common configuration options:

- [`Functions::with_jwt_secret`] - JWT signing secret for token validation
- [`Functions::with_supabase_url`] - Supabase API URL (Kong gateway)
- [`Functions::with_anon_key`] - Anonymous JWT key for public access
- [`Functions::with_service_role_key`] - Service role JWT (bypasses RLS)
- [`Functions::with_db_url`] - PostgreSQL connection string
- [`Functions::with_verify_jwt`] - Enable/disable JWT verification
- [`Functions::with_main_service_path`] - Functions directory path

See the struct documentation for the full list of options.
*/

use std::borrow::Cow;
use std::collections::BTreeMap;

use testcontainers_modules::testcontainers::core::{
    ContainerPort, ContainerState, ExecCommand, WaitFor,
};
use testcontainers_modules::testcontainers::{Image, TestcontainersError};

/// Default image name for Supabase Edge Functions
const NAME: &str = "supabase/edge-runtime";
/// Default image tag version
const TAG: &str = "v1.67.4";
/// Default port for Supabase Edge Functions
pub const FUNCTIONS_PORT: u16 = 9000;
/// Default path for functions inside the container
const DEFAULT_MAIN_SERVICE_PATH: &str = "/home/deno/functions";

/// Supabase Edge Functions container for integration testing.
///
/// This struct implements the [`Image`] trait from testcontainers, allowing you to
/// start a fully configured Supabase Edge Functions runtime for testing serverless
/// functions written in JavaScript, TypeScript, or WASM.
///
/// # Default Configuration
///
/// The default configuration includes:
/// - Port set to 9000
/// - JWT verification enabled (secure by default)
/// - Main service path at "/home/deno/functions"
///
/// # Example
///
/// ```rust,no_run
/// use supabase_testcontainers_modules::Functions;
///
/// let functions = Functions::default()
///     .with_jwt_secret("super-secret-jwt-token-with-at-least-32-characters")
///     .with_supabase_url("http://kong:8000")
///     .with_anon_key("your-anon-key")
///     .with_service_role_key("your-service-role-key")
///     .with_verify_jwt(false); // Disable for testing
/// ```
#[derive(Debug, Clone)]
pub struct Functions {
    /// Environment variables to be passed to the container
    env_vars: BTreeMap<String, String>,
    /// Docker image tag version
    tag: String,
    /// Path to the main service (functions directory) inside the container
    main_service_path: String,
}

impl Functions {
    /// Creates a new Functions instance with default configuration.
    pub fn new() -> Self {
        Self::default()
    }

    /// Creates a new Functions instance with custom environment variables.
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

    /// Sets the JWT secret for token validation.
    ///
    /// This secret is used to verify JWT tokens from clients
    /// invoking edge functions. Should be at least 32 characters.
    pub fn with_jwt_secret(mut self, secret: impl Into<String>) -> Self {
        self.env_vars
            .insert("JWT_SECRET".to_string(), secret.into());
        self
    }

    /// Sets the Supabase API URL.
    ///
    /// This is typically the Kong gateway URL that provides access
    /// to other Supabase services. Format: `http://kong:8000`
    pub fn with_supabase_url(mut self, url: impl Into<String>) -> Self {
        self.env_vars.insert("SUPABASE_URL".to_string(), url.into());
        self
    }

    /// Sets the anonymous JWT key.
    ///
    /// This key is used for unauthenticated access to Supabase services
    /// from within edge functions.
    pub fn with_anon_key(mut self, key: impl Into<String>) -> Self {
        self.env_vars
            .insert("SUPABASE_ANON_KEY".to_string(), key.into());
        self
    }

    /// Sets the service role JWT key.
    ///
    /// This key bypasses Row Level Security and should only be used
    /// for administrative operations within edge functions.
    pub fn with_service_role_key(mut self, key: impl Into<String>) -> Self {
        self.env_vars
            .insert("SUPABASE_SERVICE_ROLE_KEY".to_string(), key.into());
        self
    }

    /// Sets the PostgreSQL database connection URL.
    ///
    /// This allows edge functions to connect directly to the database.
    /// Format: `postgres://user:password@host:port/database`
    pub fn with_db_url(mut self, url: impl Into<String>) -> Self {
        self.env_vars
            .insert("SUPABASE_DB_URL".to_string(), url.into());
        self
    }

    /// Enables or disables JWT verification for function invocations.
    ///
    /// When enabled, all function invocations must include a valid JWT token.
    /// Disable for testing without authentication.
    /// Default is true (enabled).
    pub fn with_verify_jwt(mut self, verify: bool) -> Self {
        self.env_vars
            .insert("VERIFY_JWT".to_string(), verify.to_string());
        self
    }

    /// Sets the path to the main service (functions directory).
    ///
    /// This is the directory inside the container where functions are loaded from.
    /// Default is "/home/deno/functions".
    pub fn with_main_service_path(mut self, path: impl Into<String>) -> Self {
        self.main_service_path = path.into();
        self
    }

    /// Sets the server port.
    ///
    /// Default is 9000.
    pub fn with_port(mut self, port: u16) -> Self {
        self.env_vars.insert("PORT".to_string(), port.to_string());
        self
    }

    /// Sets the worker timeout in milliseconds.
    ///
    /// Maximum time a function can run before being terminated.
    pub fn with_worker_timeout_ms(mut self, timeout: u64) -> Self {
        self.env_vars
            .insert("WORKER_TIMEOUT_MS".to_string(), timeout.to_string());
        self
    }

    /// Sets the maximum number of concurrent workers.
    pub fn with_max_parallelism(mut self, max: u32) -> Self {
        self.env_vars
            .insert("MAX_PARALLELISM".to_string(), max.to_string());
        self
    }

    /// Sets a custom Docker image tag/version.
    pub fn with_tag(mut self, tag: impl Into<String>) -> Self {
        self.tag = tag.into();
        self
    }

    /// Adds a custom environment variable.
    ///
    /// Use this for Edge Functions configuration options not covered by other methods,
    /// or to pass custom environment variables accessible via `Deno.env.get()`.
    pub fn with_env(mut self, key: impl Into<String>, value: impl Into<String>) -> Self {
        self.env_vars.insert(key.into(), value.into());
        self
    }
}

impl Default for Functions {
    fn default() -> Self {
        let mut env_vars = BTreeMap::new();

        // Server configuration
        env_vars.insert("PORT".to_string(), FUNCTIONS_PORT.to_string());

        // Security configuration - JWT verification enabled by default
        env_vars.insert("VERIFY_JWT".to_string(), "true".to_string());

        Self {
            env_vars,
            tag: TAG.to_string(),
            main_service_path: DEFAULT_MAIN_SERVICE_PATH.to_string(),
        }
    }
}

impl Image for Functions {
    fn name(&self) -> &str {
        NAME
    }

    fn tag(&self) -> &str {
        &self.tag
    }

    fn ready_conditions(&self) -> Vec<WaitFor> {
        // Edge runtime logs when the server is ready to accept connections
        vec![WaitFor::message_on_stdout("Listening on")]
    }

    fn expose_ports(&self) -> &[ContainerPort] {
        &[ContainerPort::Tcp(FUNCTIONS_PORT)]
    }

    fn env_vars(
        &self,
    ) -> impl IntoIterator<Item = (impl Into<Cow<'_, str>>, impl Into<Cow<'_, str>>)> {
        &self.env_vars
    }

    fn cmd(&self) -> impl IntoIterator<Item = impl Into<Cow<'_, str>>> {
        vec![
            "start".to_string(),
            "--main-service".to_string(),
            self.main_service_path.clone(),
        ]
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
#[cfg(feature = "functions")]
mod tests {
    use super::*;
    use std::borrow::Cow;
    use testcontainers_modules::testcontainers::Image;

    #[test]
    fn test_default_configuration() {
        let functions = Functions::default();

        // Verify essential env vars are set
        assert_eq!(functions.env_vars.get("PORT"), Some(&"9000".to_string()));
        assert_eq!(
            functions.env_vars.get("VERIFY_JWT"),
            Some(&"true".to_string())
        );
        assert_eq!(
            functions.main_service_path,
            "/home/deno/functions".to_string()
        );
    }

    #[test]
    fn test_name_returns_correct_image() {
        let functions = Functions::default();
        assert_eq!(functions.name(), "supabase/edge-runtime");
    }

    #[test]
    fn test_tag_returns_correct_version() {
        let functions = Functions::default();
        assert_eq!(functions.tag(), TAG);
    }

    #[test]
    fn test_functions_port_constant() {
        assert_eq!(FUNCTIONS_PORT, 9000);
    }

    #[test]
    fn test_with_jwt_secret() {
        let functions =
            Functions::default().with_jwt_secret("super-secret-jwt-token-with-at-least-32-chars");
        assert_eq!(
            functions.env_vars.get("JWT_SECRET"),
            Some(&"super-secret-jwt-token-with-at-least-32-chars".to_string())
        );
    }

    #[test]
    fn test_with_supabase_url() {
        let functions = Functions::default().with_supabase_url("http://kong:8000");
        assert_eq!(
            functions.env_vars.get("SUPABASE_URL"),
            Some(&"http://kong:8000".to_string())
        );
    }

    #[test]
    fn test_with_anon_key() {
        let functions = Functions::default().with_anon_key("anon-jwt-token");
        assert_eq!(
            functions.env_vars.get("SUPABASE_ANON_KEY"),
            Some(&"anon-jwt-token".to_string())
        );
    }

    #[test]
    fn test_with_service_role_key() {
        let functions = Functions::default().with_service_role_key("service-role-jwt-token");
        assert_eq!(
            functions.env_vars.get("SUPABASE_SERVICE_ROLE_KEY"),
            Some(&"service-role-jwt-token".to_string())
        );
    }

    #[test]
    fn test_with_db_url() {
        let functions = Functions::default().with_db_url("postgres://user:pass@localhost:5432/db");
        assert_eq!(
            functions.env_vars.get("SUPABASE_DB_URL"),
            Some(&"postgres://user:pass@localhost:5432/db".to_string())
        );
    }

    #[test]
    fn test_with_verify_jwt() {
        let functions = Functions::default().with_verify_jwt(false);
        assert_eq!(
            functions.env_vars.get("VERIFY_JWT"),
            Some(&"false".to_string())
        );
    }

    #[test]
    fn test_with_main_service_path() {
        let functions = Functions::default().with_main_service_path("/custom/functions/path");
        assert_eq!(
            functions.main_service_path,
            "/custom/functions/path".to_string()
        );
    }

    #[test]
    fn test_with_port() {
        let functions = Functions::default().with_port(8080);
        assert_eq!(functions.env_vars.get("PORT"), Some(&"8080".to_string()));
    }

    #[test]
    fn test_with_worker_timeout_ms() {
        let functions = Functions::default().with_worker_timeout_ms(30000);
        assert_eq!(
            functions.env_vars.get("WORKER_TIMEOUT_MS"),
            Some(&"30000".to_string())
        );
    }

    #[test]
    fn test_with_max_parallelism() {
        let functions = Functions::default().with_max_parallelism(4);
        assert_eq!(
            functions.env_vars.get("MAX_PARALLELISM"),
            Some(&"4".to_string())
        );
    }

    #[test]
    fn test_with_tag_overrides_default() {
        let functions = Functions::default().with_tag("v1.0.0");
        assert_eq!(functions.tag(), "v1.0.0");
    }

    #[test]
    fn test_with_env_adds_custom_variable() {
        let functions = Functions::default()
            .with_env("CUSTOM_VAR", "custom_value")
            .with_env("ANOTHER_VAR", "another_value");

        assert_eq!(
            functions.env_vars.get("CUSTOM_VAR"),
            Some(&"custom_value".to_string())
        );
        assert_eq!(
            functions.env_vars.get("ANOTHER_VAR"),
            Some(&"another_value".to_string())
        );
    }

    #[test]
    fn test_builder_method_chaining() {
        let functions = Functions::default()
            .with_jwt_secret("my-jwt-secret")
            .with_supabase_url("http://kong:8000")
            .with_anon_key("anon-key")
            .with_service_role_key("service-key")
            .with_db_url("postgres://user:pass@localhost:5432/db")
            .with_verify_jwt(false)
            .with_main_service_path("/custom/path")
            .with_tag("v2.0.0");

        assert_eq!(
            functions.env_vars.get("JWT_SECRET"),
            Some(&"my-jwt-secret".to_string())
        );
        assert_eq!(
            functions.env_vars.get("SUPABASE_URL"),
            Some(&"http://kong:8000".to_string())
        );
        assert_eq!(
            functions.env_vars.get("SUPABASE_ANON_KEY"),
            Some(&"anon-key".to_string())
        );
        assert_eq!(
            functions.env_vars.get("SUPABASE_SERVICE_ROLE_KEY"),
            Some(&"service-key".to_string())
        );
        assert_eq!(
            functions.env_vars.get("SUPABASE_DB_URL"),
            Some(&"postgres://user:pass@localhost:5432/db".to_string())
        );
        assert_eq!(
            functions.env_vars.get("VERIFY_JWT"),
            Some(&"false".to_string())
        );
        assert_eq!(functions.main_service_path, "/custom/path".to_string());
        assert_eq!(functions.tag(), "v2.0.0");
    }

    #[test]
    fn test_new_creates_default_instance() {
        let functions = Functions::new();
        assert_eq!(functions.name(), NAME);
        assert_eq!(functions.tag(), TAG);
    }

    #[test]
    fn test_new_with_env() {
        let mut envs = BTreeMap::new();
        envs.insert("CUSTOM_KEY", "custom_value");
        envs.insert("PORT", "8080");

        let functions = Functions::new_with_env(envs);
        assert_eq!(
            functions.env_vars.get("CUSTOM_KEY"),
            Some(&"custom_value".to_string())
        );
        // Custom port should override default
        assert_eq!(functions.env_vars.get("PORT"), Some(&"8080".to_string()));
    }

    #[test]
    fn test_expose_ports() {
        let functions = Functions::default();
        let ports = functions.expose_ports();
        assert_eq!(ports.len(), 1);
        assert_eq!(ports[0], ContainerPort::Tcp(9000));
    }

    #[test]
    fn test_ready_conditions() {
        let functions = Functions::default();
        let conditions = functions.ready_conditions();
        assert_eq!(conditions.len(), 1);
    }

    #[test]
    fn test_cmd_returns_correct_startup_command() {
        let functions = Functions::default();
        let cmd: Vec<Cow<'_, str>> = functions.cmd().into_iter().map(|s| s.into()).collect();
        assert_eq!(cmd.len(), 3);
        assert_eq!(cmd[0], "start");
        assert_eq!(cmd[1], "--main-service");
        assert_eq!(cmd[2], "/home/deno/functions");
    }

    #[test]
    fn test_cmd_with_custom_path() {
        let functions = Functions::default().with_main_service_path("/custom/functions");
        let cmd: Vec<Cow<'_, str>> = functions.cmd().into_iter().map(|s| s.into()).collect();
        assert_eq!(cmd[2], "/custom/functions");
    }
}
