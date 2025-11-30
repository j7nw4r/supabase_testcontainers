//! Integration tests for GraphQL (pg_graphql) container setup patterns
//!
//! These tests verify the GraphQL + PostgREST integration pattern using standard PostgreSQL.
//! Note: Full pg_graphql functionality requires the supabase/postgres image which has
//! known compatibility issues with testcontainers. These tests verify the setup patterns
//! work correctly with standard PostgreSQL.
//!
//! Run with: `cargo test --features graphql,postgrest,const --test graphql_integration`

use anyhow::Result;
use std::sync::atomic::{AtomicU64, Ordering};
use std::time::Duration;
use supabase_testcontainers_modules::{PostgREST, LOCAL_HOST, POSTGREST_PORT};
use testcontainers::runners::AsyncRunner;
use testcontainers::ImageExt;
use testcontainers_modules::postgres::Postgres;
use tokio_postgres::NoTls;

/// PostgreSQL port constant
const POSTGRES_PORT: u16 = 5432;
/// Network name for container-to-container communication
const TEST_NETWORK: &str = "graphql-test-network";
/// PostgreSQL container alias on the shared network
const POSTGRES_ALIAS: &str = "graphql-db";

/// Atomic counter for generating unique test IDs
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

/// Returns the base URL for PostgreSQL connection
fn postgres_url(port: u16) -> String {
    format!(
        "postgres://postgres:postgres@{}:{}/postgres",
        LOCAL_HOST, port
    )
}

/// Returns the base URL for PostgREST API
fn postgrest_url(port: u16) -> String {
    format!("http://{}:{}", LOCAL_HOST, port)
}

#[cfg(test)]
mod tests {
    use super::*;
    use testcontainers::ContainerAsync;

    /// Helper struct for PostgreSQL + PostgREST setup (GraphQL pattern without pg_graphql)
    struct GraphQLPatternContext {
        #[allow(dead_code)]
        postgres: ContainerAsync<Postgres>,
        #[allow(dead_code)]
        postgrest: ContainerAsync<PostgREST>,
        postgres_port: u16,
        postgrest_port: u16,
    }

    /// Sets up PostgreSQL and PostgREST for testing the GraphQL integration pattern
    async fn setup_graphql_pattern() -> Result<GraphQLPatternContext> {
        let test_id = unique_test_id();
        let network_name = format!("{}-{}", TEST_NETWORK, test_id);
        let postgres_name = format!("{}-{}", POSTGRES_ALIAS, test_id);

        // Start PostgreSQL on shared network
        let postgres = Postgres::default()
            .with_tag("15-alpine")
            .with_network(&network_name)
            .with_container_name(&postgres_name)
            .start()
            .await?;
        let postgres_port = postgres.get_host_port_ipv4(POSTGRES_PORT).await?;

        // Set up schema
        let db_url = postgres_url(postgres_port);
        setup_test_schema(&db_url).await?;

        // Connection string for PostgREST (uses container name on shared network)
        let postgrest_db_url = format!(
            "postgres://authenticator:testpass@{}:{}/postgres",
            postgres_name, POSTGRES_PORT
        );

        // Start PostgREST on same network
        // Use api as the primary schema so /todos maps to api.todos
        let postgrest = PostgREST::default()
            .with_postgres_connection(&postgrest_db_url)
            .with_db_schemas("api")
            .with_db_anon_role("anon")
            .with_startup_timeout(Duration::from_secs(60))
            .with_network(&network_name)
            .start()
            .await?;
        let postgrest_port = postgrest.get_host_port_ipv4(POSTGREST_PORT).await?;

        // Wait for PostgREST to connect to database
        tokio::time::sleep(Duration::from_secs(2)).await;

        Ok(GraphQLPatternContext {
            postgres,
            postgrest,
            postgres_port,
            postgrest_port,
        })
    }

    /// Sets up the database schema for testing
    async fn setup_test_schema(db_url: &str) -> Result<()> {
        let (client, connection) = tokio_postgres::connect(db_url, NoTls).await?;

        tokio::spawn(async move {
            if let Err(e) = connection.await {
                eprintln!("connection error: {}", e);
            }
        });

        // Create roles for PostgREST
        client
            .batch_execute(
                r#"
            -- Create roles
            DO $$
            BEGIN
                IF NOT EXISTS (SELECT FROM pg_roles WHERE rolname = 'anon') THEN
                    CREATE ROLE anon NOLOGIN;
                END IF;
                IF NOT EXISTS (SELECT FROM pg_roles WHERE rolname = 'authenticator') THEN
                    CREATE ROLE authenticator LOGIN PASSWORD 'testpass' NOINHERIT;
                END IF;
            END
            $$;
            GRANT anon TO authenticator;

            -- Create API schema
            CREATE SCHEMA IF NOT EXISTS api;

            -- Create test table
            CREATE TABLE IF NOT EXISTS api.todos (
                id SERIAL PRIMARY KEY,
                task TEXT NOT NULL,
                done BOOLEAN DEFAULT false,
                created_at TIMESTAMPTZ DEFAULT now()
            );

            -- Insert test data
            INSERT INTO api.todos (task, done) VALUES
                ('Learn GraphQL', false),
                ('Write tests', true),
                ('Ship code', false)
            ON CONFLICT DO NOTHING;

            -- Grant permissions
            GRANT USAGE ON SCHEMA api TO anon;
            GRANT SELECT, INSERT, UPDATE, DELETE ON api.todos TO anon;
            GRANT USAGE ON SEQUENCE api.todos_id_seq TO anon;
            "#,
            )
            .await?;

        Ok(())
    }

    /// Test that PostgreSQL container starts and we can connect
    #[tokio::test]
    async fn test_postgres_container_starts() -> Result<()> {
        let test_id = unique_test_id();
        let network_name = format!("{}-{}", TEST_NETWORK, test_id);
        let postgres_name = format!("{}-{}", POSTGRES_ALIAS, test_id);

        let postgres = Postgres::default()
            .with_tag("15-alpine")
            .with_network(&network_name)
            .with_container_name(&postgres_name)
            .start()
            .await?;
        let postgres_port = postgres.get_host_port_ipv4(POSTGRES_PORT).await?;

        assert!(postgres_port > 0);

        // Test connection
        let db_url = postgres_url(postgres_port);
        let (client, connection) = tokio_postgres::connect(&db_url, NoTls).await?;

        tokio::spawn(async move {
            if let Err(e) = connection.await {
                eprintln!("connection error: {}", e);
            }
        });

        let rows = client.query("SELECT 1 as value", &[]).await?;
        assert_eq!(rows.len(), 1);
        assert_eq!(rows[0].get::<_, i32>(0), 1);

        Ok(())
    }

    /// Test that PostgREST can connect to PostgreSQL
    #[tokio::test]
    async fn test_postgrest_connects_to_postgres() -> Result<()> {
        let ctx = setup_graphql_pattern().await?;

        // Check PostgREST root endpoint
        let client = reqwest::Client::new();
        let response = client
            .get(format!("{}/", postgrest_url(ctx.postgrest_port)))
            .send()
            .await?;

        assert!(
            response.status().is_success(),
            "Expected success, got: {}",
            response.status()
        );

        Ok(())
    }

    /// Test REST API query works (pattern used for GraphQL integration)
    #[tokio::test]
    async fn test_rest_api_query() -> Result<()> {
        let ctx = setup_graphql_pattern().await?;

        // Query todos via REST API
        let client = reqwest::Client::new();
        let response = client
            .get(format!("{}/todos", postgrest_url(ctx.postgrest_port)))
            .send()
            .await?;

        assert!(
            response.status().is_success(),
            "Expected success, got: {}",
            response.status()
        );

        let body: serde_json::Value = response.json().await?;
        assert!(body.is_array(), "Expected array response");
        assert!(
            body.as_array().unwrap().len() >= 3,
            "Expected at least 3 todos"
        );

        Ok(())
    }

    /// Test REST API insert works (pattern used for GraphQL mutations)
    #[tokio::test]
    async fn test_rest_api_insert() -> Result<()> {
        let ctx = setup_graphql_pattern().await?;

        // Insert via REST API
        let client = reqwest::Client::new();
        let response = client
            .post(format!("{}/todos", postgrest_url(ctx.postgrest_port)))
            .header("Content-Type", "application/json")
            .header("Prefer", "return=representation")
            .json(&serde_json::json!({
                "task": "New task from test",
                "done": false
            }))
            .send()
            .await?;

        assert!(
            response.status().is_success(),
            "Expected success, got: {}",
            response.status()
        );

        let body: serde_json::Value = response.json().await?;
        assert!(body.is_array(), "Expected array response");
        assert_eq!(body[0]["task"], "New task from test");

        Ok(())
    }

    /// Test direct database connection works alongside PostgREST
    #[tokio::test]
    async fn test_direct_db_access() -> Result<()> {
        let ctx = setup_graphql_pattern().await?;

        // Connect directly to PostgreSQL
        let db_url = postgres_url(ctx.postgres_port);
        let (client, connection) = tokio_postgres::connect(&db_url, NoTls).await?;

        tokio::spawn(async move {
            if let Err(e) = connection.await {
                eprintln!("connection error: {}", e);
            }
        });

        // Query data
        let rows = client.query("SELECT COUNT(*) FROM api.todos", &[]).await?;
        let count: i64 = rows[0].get(0);
        assert!(count >= 3, "Expected at least 3 todos, got {}", count);

        Ok(())
    }

    /// Test OpenAPI schema is available
    #[tokio::test]
    async fn test_openapi_schema() -> Result<()> {
        let ctx = setup_graphql_pattern().await?;

        let client = reqwest::Client::new();
        let response = client
            .get(format!("{}/", postgrest_url(ctx.postgrest_port)))
            .header("Accept", "application/openapi+json")
            .send()
            .await?;

        assert!(
            response.status().is_success(),
            "Expected success, got: {}",
            response.status()
        );

        let body: serde_json::Value = response.json().await?;
        assert!(body.get("openapi").is_some() || body.get("swagger").is_some());

        Ok(())
    }
}
