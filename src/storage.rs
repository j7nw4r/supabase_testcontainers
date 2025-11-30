/*! Supabase Storage container management module.

This module provides a testcontainer implementation for [Supabase Storage](https://supabase.com/docs/guides/storage),
enabling integration testing with a real Storage API server.

# Features

- Full configuration via fluent builder API
- Support for local file storage and S3 backends
- JWT authentication for secure file access
- Configurable file size limits and signed URLs
- Multi-tenant support

# Example

```rust,no_run
use supabase_testcontainers_modules::{Storage, STORAGE_PORT, DOCKER_INTERNAL_HOST};
use testcontainers::runners::AsyncRunner;
use testcontainers_modules::postgres::Postgres;

#[tokio::main]
async fn main() -> anyhow::Result<()> {
    // 1. Start PostgreSQL
    let postgres = Postgres::default().start().await?;
    let pg_port = postgres.get_host_port_ipv4(5432).await?;

    // 2. Configure connection for Storage
    let db_url = format!(
        "postgres://postgres:postgres@{}:{}/postgres",
        DOCKER_INTERNAL_HOST, pg_port
    );

    // 3. Start Storage with local file backend
    let storage = Storage::default()
        .with_database_url(&db_url)
        .with_anon_key("your-anon-key")
        .with_service_key("your-service-key")
        .with_file_size_limit(104857600) // 100MB
        .start()
        .await?;

    let port = storage.get_host_port_ipv4(STORAGE_PORT).await?;
    println!("Storage running at http://localhost:{}", port);

    Ok(())
}
```

# Configuration

The [`Storage`] struct provides builder methods for common configuration options:

- [`Storage::with_database_url`] - PostgreSQL connection string
- [`Storage::with_anon_key`] - Anonymous JWT for public access
- [`Storage::with_service_key`] - Service role JWT (bypasses RLS)
- [`Storage::with_storage_backend`] - Backend type ("file" or "s3")
- [`Storage::with_file_size_limit`] - Maximum upload size
- [`Storage::with_global_s3_bucket`] - S3 bucket name

See the struct documentation for the full list of options.
*/

use std::borrow::Cow;
use std::collections::BTreeMap;

use testcontainers_modules::testcontainers::core::{
    ContainerPort, ContainerState, ExecCommand, WaitFor,
};
use testcontainers_modules::testcontainers::{Image, TestcontainersError};

/// Default image name for Supabase Storage
const NAME: &str = "supabase/storage-api";
/// Default image tag version
const TAG: &str = "v1.11.1";
/// Default port for Supabase Storage API
pub const STORAGE_PORT: u16 = 5000;

/// Supabase Storage container for integration testing.
///
/// This struct implements the [`Image`] trait from testcontainers, allowing you to
/// start a fully configured Supabase Storage service for testing file uploads and management.
///
/// # Default Configuration
///
/// The default configuration includes:
/// - Storage backend set to "file" (local filesystem)
/// - File storage path at "/var/lib/storage"
/// - File size limit of 50MB (52428800 bytes)
/// - Region set to "local"
/// - Multi-tenant mode disabled
///
/// # Example
///
/// ```rust,no_run
/// use supabase_testcontainers_modules::Storage;
///
/// let storage = Storage::default()
///     .with_database_url("postgres://user:pass@host:5432/db")
///     .with_anon_key("anon-jwt-token")
///     .with_service_key("service-role-jwt-token")
///     .with_file_size_limit(104857600); // 100MB
/// ```
#[derive(Debug, Clone)]
pub struct Storage {
    /// Environment variables to be passed to the container
    env_vars: BTreeMap<String, String>,
    /// Docker image tag version
    tag: String,
}

impl Storage {
    /// Creates a new Storage instance
    pub fn new() -> Self {
        Self::default()
    }

    /// Creates a new Storage instance with custom environment variables
    pub fn new_with_env(envs: BTreeMap<&str, &str>) -> Self {
        let mut instance = Self::default();
        for (key, val) in envs {
            instance.env_vars.insert(key.to_string(), val.to_string());
        }
        instance
    }

    /// Sets the PostgreSQL database connection URL
    pub fn with_database_url(mut self, url: impl Into<String>) -> Self {
        self.env_vars.insert("DATABASE_URL".to_string(), url.into());
        self
    }

    /// Sets the storage backend type
    ///
    /// Valid values: "file", "s3"
    pub fn with_storage_backend(mut self, backend: impl Into<String>) -> Self {
        self.env_vars
            .insert("STORAGE_BACKEND".to_string(), backend.into());
        self
    }

    /// Sets the anonymous JWT key for unauthenticated access
    pub fn with_anon_key(mut self, key: impl Into<String>) -> Self {
        self.env_vars.insert("ANON_KEY".to_string(), key.into());
        self
    }

    /// Sets the service role JWT key for privileged access (bypasses RLS)
    pub fn with_service_key(mut self, key: impl Into<String>) -> Self {
        self.env_vars.insert("SERVICE_KEY".to_string(), key.into());
        self
    }

    /// Sets the JWT secret for token validation
    pub fn with_jwt_secret(mut self, secret: impl Into<String>) -> Self {
        self.env_vars
            .insert("PGRST_JWT_SECRET".to_string(), secret.into());
        self
    }

    /// Sets the PostgREST server URL for database operations
    pub fn with_postgrest_url(mut self, url: impl Into<String>) -> Self {
        self.env_vars
            .insert("POSTGREST_URL".to_string(), url.into());
        self
    }

    /// Sets the storage tenant identifier
    pub fn with_tenant_id(mut self, tenant_id: impl Into<String>) -> Self {
        self.env_vars
            .insert("TENANT_ID".to_string(), tenant_id.into());
        self
    }

    /// Sets the storage region (for S3-compatible backends)
    pub fn with_region(mut self, region: impl Into<String>) -> Self {
        self.env_vars.insert("REGION".to_string(), region.into());
        self
    }

    /// Sets the S3 bucket name
    pub fn with_global_s3_bucket(mut self, bucket: impl Into<String>) -> Self {
        self.env_vars
            .insert("GLOBAL_S3_BUCKET".to_string(), bucket.into());
        self
    }

    /// Sets the maximum file size limit in bytes
    ///
    /// Default is 52428800 (50MB)
    pub fn with_file_size_limit(mut self, limit: u64) -> Self {
        self.env_vars
            .insert("FILE_SIZE_LIMIT".to_string(), limit.to_string());
        self
    }

    /// Sets the local file storage path (when using "file" backend)
    pub fn with_file_storage_path(mut self, path: impl Into<String>) -> Self {
        self.env_vars
            .insert("FILE_STORAGE_BACKEND_PATH".to_string(), path.into());
        self
    }

    /// Sets the expiration time for upload signed URLs in seconds
    pub fn with_upload_signed_url_expiration(mut self, seconds: u32) -> Self {
        self.env_vars.insert(
            "UPLOAD_SIGNED_URL_EXPIRATION_TIME".to_string(),
            seconds.to_string(),
        );
        self
    }

    /// Enables or disables multi-tenant mode
    pub fn with_multitenant(mut self, enabled: bool) -> Self {
        self.env_vars
            .insert("IS_MULTITENANT".to_string(), enabled.to_string());
        self
    }

    /// Sets the image resumable uploads configuration
    ///
    /// Enables TUS protocol support for resumable uploads
    pub fn with_tus_url_path(mut self, path: impl Into<String>) -> Self {
        self.env_vars
            .insert("TUS_URL_PATH".to_string(), path.into());
        self
    }

    /// Sets a custom Docker image tag/version
    pub fn with_tag(mut self, tag: impl Into<String>) -> Self {
        self.tag = tag.into();
        self
    }

    /// Adds a custom environment variable
    ///
    /// Use this for Storage configuration options not covered by other methods.
    pub fn with_env(mut self, key: impl Into<String>, value: impl Into<String>) -> Self {
        self.env_vars.insert(key.into(), value.into());
        self
    }
}

impl Default for Storage {
    fn default() -> Self {
        let mut env_vars = BTreeMap::new();

        // Server configuration
        env_vars.insert("PORT".to_string(), STORAGE_PORT.to_string());
        env_vars.insert("REGION".to_string(), "local".to_string());

        // Storage configuration
        env_vars.insert("STORAGE_BACKEND".to_string(), "file".to_string());
        env_vars.insert(
            "FILE_STORAGE_BACKEND_PATH".to_string(),
            "/var/lib/storage".to_string(),
        );
        env_vars.insert("FILE_SIZE_LIMIT".to_string(), "52428800".to_string()); // 50MB
        env_vars.insert("GLOBAL_S3_BUCKET".to_string(), "storage".to_string());

        // Tenant configuration
        env_vars.insert("TENANT_ID".to_string(), "default".to_string());
        env_vars.insert("IS_MULTITENANT".to_string(), "false".to_string());

        Self {
            env_vars,
            tag: TAG.to_string(),
        }
    }
}

impl Image for Storage {
    fn name(&self) -> &str {
        NAME
    }

    fn tag(&self) -> &str {
        &self.tag
    }

    fn ready_conditions(&self) -> Vec<WaitFor> {
        // Storage-api logs JSON format: {"msg":"[Server] Started Successfully",...}
        vec![WaitFor::message_on_stdout("[Server] Started Successfully")]
    }

    fn expose_ports(&self) -> &[ContainerPort] {
        &[ContainerPort::Tcp(STORAGE_PORT)]
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
#[cfg(feature = "storage")]
mod tests {
    use super::*;
    use testcontainers_modules::testcontainers::Image;

    #[test]
    fn test_default_configuration() {
        let storage = Storage::default();

        // Verify essential env vars are set
        assert_eq!(
            storage.env_vars.get("STORAGE_BACKEND"),
            Some(&"file".to_string())
        );
        assert_eq!(storage.env_vars.get("REGION"), Some(&"local".to_string()));
        assert_eq!(storage.env_vars.get("PORT"), Some(&"5000".to_string()));
        assert_eq!(
            storage.env_vars.get("FILE_STORAGE_BACKEND_PATH"),
            Some(&"/var/lib/storage".to_string())
        );
        assert_eq!(
            storage.env_vars.get("FILE_SIZE_LIMIT"),
            Some(&"52428800".to_string())
        );
        assert_eq!(
            storage.env_vars.get("TENANT_ID"),
            Some(&"default".to_string())
        );
        assert_eq!(
            storage.env_vars.get("IS_MULTITENANT"),
            Some(&"false".to_string())
        );
    }

    #[test]
    fn test_name_returns_correct_image() {
        let storage = Storage::default();
        assert_eq!(storage.name(), "supabase/storage-api");
    }

    #[test]
    fn test_tag_returns_correct_version() {
        let storage = Storage::default();
        assert_eq!(storage.tag(), "v1.11.1");
    }

    #[test]
    fn test_storage_port_constant() {
        assert_eq!(STORAGE_PORT, 5000);
    }

    #[test]
    fn test_with_database_url() {
        let storage =
            Storage::default().with_database_url("postgres://user:pass@localhost:5432/db");
        assert_eq!(
            storage.env_vars.get("DATABASE_URL"),
            Some(&"postgres://user:pass@localhost:5432/db".to_string())
        );
    }

    #[test]
    fn test_with_storage_backend() {
        let storage = Storage::default().with_storage_backend("s3");
        assert_eq!(
            storage.env_vars.get("STORAGE_BACKEND"),
            Some(&"s3".to_string())
        );
    }

    #[test]
    fn test_with_anon_key() {
        let storage = Storage::default().with_anon_key("anon-jwt-token");
        assert_eq!(
            storage.env_vars.get("ANON_KEY"),
            Some(&"anon-jwt-token".to_string())
        );
    }

    #[test]
    fn test_with_service_key() {
        let storage = Storage::default().with_service_key("service-role-jwt-token");
        assert_eq!(
            storage.env_vars.get("SERVICE_KEY"),
            Some(&"service-role-jwt-token".to_string())
        );
    }

    #[test]
    fn test_with_jwt_secret() {
        let storage = Storage::default().with_jwt_secret("my-jwt-secret");
        assert_eq!(
            storage.env_vars.get("PGRST_JWT_SECRET"),
            Some(&"my-jwt-secret".to_string())
        );
    }

    #[test]
    fn test_with_postgrest_url() {
        let storage = Storage::default().with_postgrest_url("http://postgrest:3000");
        assert_eq!(
            storage.env_vars.get("POSTGREST_URL"),
            Some(&"http://postgrest:3000".to_string())
        );
    }

    #[test]
    fn test_with_tenant_id() {
        let storage = Storage::default().with_tenant_id("my-tenant");
        assert_eq!(
            storage.env_vars.get("TENANT_ID"),
            Some(&"my-tenant".to_string())
        );
    }

    #[test]
    fn test_with_region() {
        let storage = Storage::default().with_region("us-east-1");
        assert_eq!(
            storage.env_vars.get("REGION"),
            Some(&"us-east-1".to_string())
        );
    }

    #[test]
    fn test_with_global_s3_bucket() {
        let storage = Storage::default().with_global_s3_bucket("my-bucket");
        assert_eq!(
            storage.env_vars.get("GLOBAL_S3_BUCKET"),
            Some(&"my-bucket".to_string())
        );
    }

    #[test]
    fn test_with_file_size_limit() {
        let storage = Storage::default().with_file_size_limit(104857600); // 100MB
        assert_eq!(
            storage.env_vars.get("FILE_SIZE_LIMIT"),
            Some(&"104857600".to_string())
        );
    }

    #[test]
    fn test_with_file_storage_path() {
        let storage = Storage::default().with_file_storage_path("/data/storage");
        assert_eq!(
            storage.env_vars.get("FILE_STORAGE_BACKEND_PATH"),
            Some(&"/data/storage".to_string())
        );
    }

    #[test]
    fn test_with_upload_signed_url_expiration() {
        let storage = Storage::default().with_upload_signed_url_expiration(3600);
        assert_eq!(
            storage.env_vars.get("UPLOAD_SIGNED_URL_EXPIRATION_TIME"),
            Some(&"3600".to_string())
        );
    }

    #[test]
    fn test_with_multitenant() {
        let storage = Storage::default().with_multitenant(true);
        assert_eq!(
            storage.env_vars.get("IS_MULTITENANT"),
            Some(&"true".to_string())
        );
    }

    #[test]
    fn test_with_tus_url_path() {
        let storage = Storage::default().with_tus_url_path("/upload/resumable");
        assert_eq!(
            storage.env_vars.get("TUS_URL_PATH"),
            Some(&"/upload/resumable".to_string())
        );
    }

    #[test]
    fn test_with_tag_overrides_default() {
        let storage = Storage::default().with_tag("v1.0.0");
        assert_eq!(storage.tag(), "v1.0.0");
    }

    #[test]
    fn test_with_env_adds_custom_variable() {
        let storage = Storage::default()
            .with_env("CUSTOM_VAR", "custom_value")
            .with_env("ANOTHER_VAR", "another_value");

        assert_eq!(
            storage.env_vars.get("CUSTOM_VAR"),
            Some(&"custom_value".to_string())
        );
        assert_eq!(
            storage.env_vars.get("ANOTHER_VAR"),
            Some(&"another_value".to_string())
        );
    }

    #[test]
    fn test_builder_method_chaining() {
        let storage = Storage::default()
            .with_database_url("postgres://user:pass@localhost:5432/db")
            .with_anon_key("anon-key")
            .with_service_key("service-key")
            .with_storage_backend("s3")
            .with_region("us-west-2")
            .with_global_s3_bucket("my-bucket")
            .with_file_size_limit(200000000)
            .with_tag("v1.0.0");

        assert_eq!(
            storage.env_vars.get("DATABASE_URL"),
            Some(&"postgres://user:pass@localhost:5432/db".to_string())
        );
        assert_eq!(
            storage.env_vars.get("ANON_KEY"),
            Some(&"anon-key".to_string())
        );
        assert_eq!(
            storage.env_vars.get("SERVICE_KEY"),
            Some(&"service-key".to_string())
        );
        assert_eq!(
            storage.env_vars.get("STORAGE_BACKEND"),
            Some(&"s3".to_string())
        );
        assert_eq!(
            storage.env_vars.get("REGION"),
            Some(&"us-west-2".to_string())
        );
        assert_eq!(
            storage.env_vars.get("GLOBAL_S3_BUCKET"),
            Some(&"my-bucket".to_string())
        );
        assert_eq!(
            storage.env_vars.get("FILE_SIZE_LIMIT"),
            Some(&"200000000".to_string())
        );
        assert_eq!(storage.tag(), "v1.0.0");
    }

    #[test]
    fn test_new_creates_default_instance() {
        let storage = Storage::new();
        assert_eq!(storage.name(), NAME);
        assert_eq!(storage.tag(), TAG);
    }

    #[test]
    fn test_expose_ports() {
        let storage = Storage::default();
        let ports = storage.expose_ports();
        assert_eq!(ports.len(), 1);
        assert_eq!(ports[0], ContainerPort::Tcp(5000));
    }

    #[test]
    fn test_ready_conditions() {
        let storage = Storage::default();
        let conditions = storage.ready_conditions();
        assert_eq!(conditions.len(), 1);
    }
}
