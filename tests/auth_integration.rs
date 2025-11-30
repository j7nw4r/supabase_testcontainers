//! Integration tests for Supabase Auth container with PostgreSQL
//!
//! These tests require Docker to be running and will start real containers.
//! Run with: `cargo test --features auth,const --test auth_integration`

use anyhow::Result;
use std::sync::atomic::{AtomicU64, Ordering};
use supabase_testcontainers_modules::{Auth, AUTH_PORT, LOCAL_HOST};
use testcontainers::runners::AsyncRunner;
use testcontainers::{ContainerAsync, ImageExt};
use testcontainers_modules::postgres::Postgres;
use tokio_postgres::NoTls;

/// PostgreSQL port constant
const POSTGRES_PORT: u16 = 5432;
/// Network name for container-to-container communication
const TEST_NETWORK: &str = "supabase-test-network";
/// PostgreSQL container alias on the shared network
const POSTGRES_ALIAS: &str = "postgres";

/// Atomic counter for generating unique test IDs (avoids race conditions)
static TEST_COUNTER: AtomicU64 = AtomicU64::new(0);

/// Generates a unique test ID combining timestamp and atomic counter
fn unique_test_id() -> String {
    let timestamp = std::time::SystemTime::now()
        .duration_since(std::time::UNIX_EPOCH)
        .unwrap()
        .as_millis();
    let counter = TEST_COUNTER.fetch_add(1, Ordering::SeqCst);
    format!("{}-{}", timestamp, counter)
}

/// Helper struct containing both containers and connection info
pub struct AuthTestContext {
    /// PostgreSQL container (must be kept alive for the duration of tests)
    pub postgres: ContainerAsync<Postgres>,
    /// Auth container (must be kept alive for the duration of tests)
    pub auth: ContainerAsync<Auth>,
    /// Host port for Auth API
    pub auth_port: u16,
    /// Host port for PostgreSQL
    pub postgres_port: u16,
}

/// Sets up PostgreSQL and Auth containers for integration testing.
///
/// This function:
/// 1. Starts a PostgreSQL 15 container (required for Auth migrations)
/// 2. Initializes the auth database schema
/// 3. Starts the Auth container connected to PostgreSQL
///
/// # Returns
/// An `AuthTestContext` containing both containers and their ports.
///
/// # Example
/// ```ignore
/// let ctx = setup_auth_with_postgres().await?;
/// // Use ctx.auth_port to make HTTP requests to the Auth API
/// // Containers are automatically stopped when ctx goes out of scope
/// ```
pub async fn setup_auth_with_postgres() -> Result<AuthTestContext> {
    // Generate unique network and container names for test isolation
    let test_id = unique_test_id();
    let network_name = format!("{}-{}", TEST_NETWORK, test_id);
    let postgres_name = format!("{}-{}", POSTGRES_ALIAS, test_id);

    // Start PostgreSQL 15 on shared network with a known container name
    let postgres = Postgres::default()
        .with_tag("15-alpine")
        .with_network(&network_name)
        .with_container_name(&postgres_name)
        .start()
        .await?;
    let postgres_port = postgres.get_host_port_ipv4(POSTGRES_PORT).await?;

    // Connection string for Auth container (uses container name on shared network)
    let auth_db_url = format!(
        "postgres://supabase_auth_admin:testpassword@{}:{}/postgres",
        postgres_name, POSTGRES_PORT
    );

    // Connection string for schema init (uses localhost from host machine)
    let local_db_url = format!(
        "postgres://postgres:postgres@{}:{}/postgres",
        LOCAL_HOST, postgres_port
    );

    // Initialize Auth with database schema and start container on same network
    let auth = Auth::default()
        .with_db_url(&auth_db_url)
        .with_mailer_autoconfirm(true)
        .with_sms_autoconfirm(true)
        .with_anonymous_users(true)
        .init_db_schema(&local_db_url, "testpassword")
        .await?
        .with_network(&network_name)
        .start()
        .await?;

    let auth_port = auth.get_host_port_ipv4(AUTH_PORT).await?;

    Ok(AuthTestContext {
        postgres,
        auth,
        auth_port,
        postgres_port,
    })
}

/// Returns the base URL for the Auth API
pub fn auth_url(port: u16) -> String {
    format!("http://{}:{}", LOCAL_HOST, port)
}

#[cfg(test)]
mod tests {
    use super::*;

    /// Test that both containers can be started together
    #[tokio::test]
    async fn test_containers_start() -> Result<()> {
        let ctx = setup_auth_with_postgres().await?;

        // Verify ports are assigned
        assert!(ctx.auth_port > 0);
        assert!(ctx.postgres_port > 0);

        Ok(())
    }

    /// Test that the health endpoint returns 200 OK
    #[tokio::test]
    async fn test_health_endpoint_returns_200() -> Result<()> {
        let ctx = setup_auth_with_postgres().await?;

        let response = reqwest::get(format!("{}/health", auth_url(ctx.auth_port))).await?;

        assert_eq!(response.status(), 200);

        Ok(())
    }

    /// Test that the settings endpoint returns configuration
    #[tokio::test]
    async fn test_settings_endpoint_returns_config() -> Result<()> {
        let ctx = setup_auth_with_postgres().await?;

        let response = reqwest::get(format!("{}/settings", auth_url(ctx.auth_port))).await?;

        assert_eq!(response.status(), 200);

        let settings: serde_json::Value = response.json().await?;

        // Verify settings contains expected fields
        assert!(settings.get("external").is_some());
        assert!(settings.get("disable_signup").is_some());

        Ok(())
    }

    /// Test that anonymous signup works when enabled
    #[tokio::test]
    async fn test_anonymous_signup() -> Result<()> {
        let ctx = setup_auth_with_postgres().await?;

        let client = reqwest::Client::new();
        let response = client
            .post(format!("{}/signup", auth_url(ctx.auth_port)))
            .header("Content-Type", "application/json")
            .json(&serde_json::json!({}))
            .send()
            .await?;

        assert!(response.status().is_success());

        let body: serde_json::Value = response.json().await?;
        assert!(body.get("access_token").is_some());

        Ok(())
    }

    /// Test that email signup with autoconfirm returns tokens
    #[tokio::test]
    async fn test_email_signup_with_autoconfirm() -> Result<()> {
        let ctx = setup_auth_with_postgres().await?;

        let client = reqwest::Client::new();
        let response = client
            .post(format!("{}/signup", auth_url(ctx.auth_port)))
            .header("Content-Type", "application/json")
            .json(&serde_json::json!({
                "email": "test@example.com",
                "password": "testpassword123"
            }))
            .send()
            .await?;

        assert!(response.status().is_success());

        let body: serde_json::Value = response.json().await?;
        // With autoconfirm enabled, user should get tokens immediately
        assert!(body.get("access_token").is_some());
        assert!(body.get("refresh_token").is_some());

        Ok(())
    }

    /// Test that signup is rejected when disabled
    #[tokio::test]
    async fn test_signup_rejected_when_disabled() -> Result<()> {
        // Generate unique network and container names for test isolation
        let test_id = unique_test_id();
        let network_name = format!("{}-{}", TEST_NETWORK, test_id);
        let postgres_name = format!("{}-{}", POSTGRES_ALIAS, test_id);

        // Start PostgreSQL 15 on shared network
        let postgres = Postgres::default()
            .with_tag("15-alpine")
            .with_network(&network_name)
            .with_container_name(&postgres_name)
            .start()
            .await?;
        let postgres_port = postgres.get_host_port_ipv4(POSTGRES_PORT).await?;

        let auth_db_url = format!(
            "postgres://supabase_auth_admin:testpassword@{}:{}/postgres",
            postgres_name, POSTGRES_PORT
        );
        let local_db_url = format!(
            "postgres://postgres:postgres@{}:{}/postgres",
            LOCAL_HOST, postgres_port
        );

        // Create Auth with signup disabled on same network
        let auth = Auth::default()
            .with_db_url(&auth_db_url)
            .with_signup_disabled(true)
            .init_db_schema(&local_db_url, "testpassword")
            .await?
            .with_network(&network_name)
            .start()
            .await?;

        let auth_port = auth.get_host_port_ipv4(AUTH_PORT).await?;

        let client = reqwest::Client::new();
        let response = client
            .post(format!("{}/signup", auth_url(auth_port)))
            .header("Content-Type", "application/json")
            .json(&serde_json::json!({
                "email": "test@example.com",
                "password": "testpassword123"
            }))
            .send()
            .await?;

        // Signup should be rejected
        assert!(!response.status().is_success());

        Ok(())
    }

    /// Test that token refresh works
    #[tokio::test]
    async fn test_token_refresh() -> Result<()> {
        let ctx = setup_auth_with_postgres().await?;

        let client = reqwest::Client::new();

        // First, sign up to get tokens
        let signup_response = client
            .post(format!("{}/signup", auth_url(ctx.auth_port)))
            .header("Content-Type", "application/json")
            .json(&serde_json::json!({
                "email": "refresh@example.com",
                "password": "testpassword123"
            }))
            .send()
            .await?;

        assert!(signup_response.status().is_success());

        let signup_body: serde_json::Value = signup_response.json().await?;
        let refresh_token = signup_body["refresh_token"]
            .as_str()
            .expect("refresh_token should be present");

        // Use refresh token to get new access token
        let refresh_response = client
            .post(format!(
                "{}/token?grant_type=refresh_token",
                auth_url(ctx.auth_port)
            ))
            .header("Content-Type", "application/json")
            .json(&serde_json::json!({
                "refresh_token": refresh_token
            }))
            .send()
            .await?;

        assert!(refresh_response.status().is_success());

        let refresh_body: serde_json::Value = refresh_response.json().await?;
        assert!(refresh_body.get("access_token").is_some());

        Ok(())
    }

    /// Test that user retrieval with access token works
    #[tokio::test]
    async fn test_user_retrieval_with_token() -> Result<()> {
        let ctx = setup_auth_with_postgres().await?;

        let client = reqwest::Client::new();

        // Sign up to get access token
        let signup_response = client
            .post(format!("{}/signup", auth_url(ctx.auth_port)))
            .header("Content-Type", "application/json")
            .json(&serde_json::json!({
                "email": "user@example.com",
                "password": "testpassword123"
            }))
            .send()
            .await?;

        assert!(signup_response.status().is_success());

        let signup_body: serde_json::Value = signup_response.json().await?;
        let access_token = signup_body["access_token"]
            .as_str()
            .expect("access_token should be present");

        // Get user info using access token
        let user_response = client
            .get(format!("{}/user", auth_url(ctx.auth_port)))
            .header("Authorization", format!("Bearer {}", access_token))
            .send()
            .await?;

        assert!(user_response.status().is_success());

        let user_body: serde_json::Value = user_response.json().await?;
        assert_eq!(user_body["email"].as_str(), Some("user@example.com"));

        Ok(())
    }

    /// Test that auth schema is created in PostgreSQL
    #[tokio::test]
    async fn test_auth_schema_created() -> Result<()> {
        let ctx = setup_auth_with_postgres().await?;

        // Connect to PostgreSQL directly
        let db_url = format!(
            "postgres://postgres:postgres@{}:{}/postgres",
            LOCAL_HOST, ctx.postgres_port
        );

        let (client, connection) = tokio_postgres::connect(&db_url, NoTls).await?;

        tokio::spawn(async move {
            if let Err(e) = connection.await {
                eprintln!("connection error: {}", e);
            }
        });

        // Check auth schema exists
        let rows = client
            .query(
                "SELECT schema_name FROM information_schema.schemata WHERE schema_name = 'auth'",
                &[],
            )
            .await?;

        assert_eq!(rows.len(), 1);
        assert_eq!(rows[0].get::<_, String>(0), "auth");

        Ok(())
    }

    /// Test that supabase_auth_admin user is created
    #[tokio::test]
    async fn test_auth_admin_user_created() -> Result<()> {
        let ctx = setup_auth_with_postgres().await?;

        // Connect to PostgreSQL directly
        let db_url = format!(
            "postgres://postgres:postgres@{}:{}/postgres",
            LOCAL_HOST, ctx.postgres_port
        );

        let (client, connection) = tokio_postgres::connect(&db_url, NoTls).await?;

        tokio::spawn(async move {
            if let Err(e) = connection.await {
                eprintln!("connection error: {}", e);
            }
        });

        // Check supabase_auth_admin user exists
        let rows = client
            .query(
                "SELECT usename FROM pg_user WHERE usename = 'supabase_auth_admin'",
                &[],
            )
            .await?;

        assert_eq!(rows.len(), 1);
        assert_eq!(rows[0].get::<_, String>(0), "supabase_auth_admin");

        Ok(())
    }

    /// Test that migrations run successfully (verified via health check)
    #[tokio::test]
    async fn test_migrations_run_successfully() -> Result<()> {
        let ctx = setup_auth_with_postgres().await?;

        // If health check passes, migrations ran successfully
        // The Auth container won't become healthy until migrations complete
        let response = reqwest::get(format!("{}/health", auth_url(ctx.auth_port))).await?;

        assert_eq!(response.status(), 200);

        // Additional verification: check that auth.users table exists
        let db_url = format!(
            "postgres://postgres:postgres@{}:{}/postgres",
            LOCAL_HOST, ctx.postgres_port
        );

        let (client, connection) = tokio_postgres::connect(&db_url, NoTls).await?;

        tokio::spawn(async move {
            if let Err(e) = connection.await {
                eprintln!("connection error: {}", e);
            }
        });

        // Check that auth.users table exists (created by migrations)
        let rows = client
            .query(
                "SELECT table_name FROM information_schema.tables
                 WHERE table_schema = 'auth' AND table_name = 'users'",
                &[],
            )
            .await?;

        assert_eq!(rows.len(), 1);

        Ok(())
    }
}
