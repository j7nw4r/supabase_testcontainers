//! Integration tests for PostgREST container
//!
//! These tests verify PostgREST functionality including health endpoints,
//! CRUD operations, JWT authentication, and row limiting.
//!
//! Run with: `cargo test --features postgrest,const --test postgrest_integration`

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
const TEST_NETWORK: &str = "postgrest-test-network";
/// PostgreSQL container alias on the shared network
const POSTGRES_ALIAS: &str = "postgrest-db";

/// JWT secret used for authentication tests
const JWT_SECRET: &str = "super-secret-jwt-token-with-at-least-32-characters-for-hs256";

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

    /// Helper struct for PostgreSQL + PostgREST setup
    struct PostgRESTContext {
        #[allow(dead_code)]
        postgres: ContainerAsync<Postgres>,
        #[allow(dead_code)]
        postgrest: ContainerAsync<PostgREST>,
        postgres_port: u16,
        postgrest_port: u16,
    }

    /// Sets up PostgreSQL and PostgREST for testing
    async fn setup_postgrest(
        jwt_secret: Option<&str>,
        max_rows: Option<u32>,
    ) -> Result<PostgRESTContext> {
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

        // Build PostgREST with optional configuration
        // Note: PostgREST methods must be called before ImageExt methods
        let mut postgrest_image = PostgREST::default()
            .with_postgres_connection(&postgrest_db_url)
            .with_db_schemas("api")
            .with_db_anon_role("anon");

        if let Some(secret) = jwt_secret {
            postgrest_image = postgrest_image.with_jwt_secret(secret);
        }

        if let Some(rows) = max_rows {
            postgrest_image = postgrest_image.with_max_rows(rows);
        }

        let postgrest = postgrest_image
            .with_startup_timeout(Duration::from_secs(60))
            .with_network(&network_name)
            .start()
            .await?;
        let postgrest_port = postgrest.get_host_port_ipv4(POSTGREST_PORT).await?;

        // Wait for PostgREST to connect to database
        tokio::time::sleep(Duration::from_secs(2)).await;

        Ok(PostgRESTContext {
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
                IF NOT EXISTS (SELECT FROM pg_roles WHERE rolname = 'authenticated') THEN
                    CREATE ROLE authenticated NOLOGIN;
                END IF;
                IF NOT EXISTS (SELECT FROM pg_roles WHERE rolname = 'authenticator') THEN
                    CREATE ROLE authenticator LOGIN PASSWORD 'testpass' NOINHERIT;
                END IF;
            END
            $$;
            GRANT anon TO authenticator;
            GRANT authenticated TO authenticator;

            -- Create API schema
            CREATE SCHEMA IF NOT EXISTS api;

            -- Create test table
            CREATE TABLE IF NOT EXISTS api.todos (
                id SERIAL PRIMARY KEY,
                task TEXT NOT NULL,
                done BOOLEAN DEFAULT false,
                created_at TIMESTAMPTZ DEFAULT now()
            );

            -- Grant permissions to anon
            GRANT USAGE ON SCHEMA api TO anon;
            GRANT SELECT, INSERT, UPDATE, DELETE ON api.todos TO anon;
            GRANT USAGE ON SEQUENCE api.todos_id_seq TO anon;

            -- Grant permissions to authenticated
            GRANT USAGE ON SCHEMA api TO authenticated;
            GRANT SELECT, INSERT, UPDATE, DELETE ON api.todos TO authenticated;
            GRANT USAGE ON SEQUENCE api.todos_id_seq TO authenticated;
            "#,
            )
            .await?;

        Ok(())
    }

    /// Inserts test data into the todos table
    async fn insert_test_data(db_url: &str, count: usize) -> Result<()> {
        let (client, connection) = tokio_postgres::connect(db_url, NoTls).await?;

        tokio::spawn(async move {
            if let Err(e) = connection.await {
                eprintln!("connection error: {}", e);
            }
        });

        for i in 0..count {
            client
                .execute(
                    "INSERT INTO api.todos (task, done) VALUES ($1, $2)",
                    &[&format!("Task {}", i), &false],
                )
                .await?;
        }

        Ok(())
    }

    /// Test that PostgreSQL and PostgREST containers start successfully
    #[tokio::test]
    async fn test_containers_start() -> Result<()> {
        let ctx = setup_postgrest(None, None).await?;

        assert!(ctx.postgres_port > 0, "PostgreSQL port should be assigned");
        assert!(ctx.postgrest_port > 0, "PostgREST port should be assigned");

        Ok(())
    }

    /// Test that PostgREST health/ready endpoint returns 200
    #[tokio::test]
    async fn test_health_endpoint_returns_200() -> Result<()> {
        let ctx = setup_postgrest(None, None).await?;

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

    /// Test that OpenAPI schema is available
    #[tokio::test]
    async fn test_openapi_schema_available() -> Result<()> {
        let ctx = setup_postgrest(None, None).await?;

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
        assert!(
            body.get("openapi").is_some() || body.get("swagger").is_some(),
            "Expected OpenAPI schema"
        );

        Ok(())
    }

    /// Test basic CRUD operations via REST API
    #[tokio::test]
    async fn test_table_crud_operations() -> Result<()> {
        let ctx = setup_postgrest(None, None).await?;
        let client = reqwest::Client::new();
        let base_url = postgrest_url(ctx.postgrest_port);

        // CREATE - Insert a new todo
        let create_response = client
            .post(format!("{}/todos", base_url))
            .header("Content-Type", "application/json")
            .header("Prefer", "return=representation")
            .json(&serde_json::json!({
                "task": "Test CRUD task",
                "done": false
            }))
            .send()
            .await?;

        assert!(
            create_response.status().is_success(),
            "CREATE failed: {}",
            create_response.status()
        );

        let created: Vec<serde_json::Value> = create_response.json().await?;
        assert_eq!(created.len(), 1);
        let created_id = created[0]["id"].as_i64().unwrap();
        assert_eq!(created[0]["task"], "Test CRUD task");

        // READ - Get the created todo
        let read_response = client
            .get(format!("{}/todos?id=eq.{}", base_url, created_id))
            .send()
            .await?;

        assert!(
            read_response.status().is_success(),
            "READ failed: {}",
            read_response.status()
        );

        let read: Vec<serde_json::Value> = read_response.json().await?;
        assert_eq!(read.len(), 1);
        assert_eq!(read[0]["task"], "Test CRUD task");

        // UPDATE - Modify the todo
        let update_response = client
            .patch(format!("{}/todos?id=eq.{}", base_url, created_id))
            .header("Content-Type", "application/json")
            .header("Prefer", "return=representation")
            .json(&serde_json::json!({
                "done": true
            }))
            .send()
            .await?;

        assert!(
            update_response.status().is_success(),
            "UPDATE failed: {}",
            update_response.status()
        );

        let updated: Vec<serde_json::Value> = update_response.json().await?;
        assert_eq!(updated[0]["done"], true);

        // DELETE - Remove the todo
        let delete_response = client
            .delete(format!("{}/todos?id=eq.{}", base_url, created_id))
            .send()
            .await?;

        assert!(
            delete_response.status().is_success(),
            "DELETE failed: {}",
            delete_response.status()
        );

        // Verify deletion
        let verify_response = client
            .get(format!("{}/todos?id=eq.{}", base_url, created_id))
            .send()
            .await?;

        let verify: Vec<serde_json::Value> = verify_response.json().await?;
        assert!(verify.is_empty(), "Todo should be deleted");

        Ok(())
    }

    /// Test that anonymous role has correct access
    #[tokio::test]
    async fn test_anonymous_role_access() -> Result<()> {
        let ctx = setup_postgrest(None, None).await?;

        // Insert test data directly via PostgreSQL
        let db_url = postgres_url(ctx.postgres_port);
        insert_test_data(&db_url, 3).await?;

        // Make request without any auth header (uses anon role)
        let client = reqwest::Client::new();
        let response = client
            .get(format!("{}/todos", postgrest_url(ctx.postgrest_port)))
            .send()
            .await?;

        assert!(
            response.status().is_success(),
            "Anonymous request should succeed: {}",
            response.status()
        );

        let body: Vec<serde_json::Value> = response.json().await?;
        assert!(body.len() >= 3, "Should be able to read todos as anon");

        Ok(())
    }

    /// Test JWT authentication and role switching
    #[tokio::test]
    async fn test_jwt_authentication() -> Result<()> {
        let ctx = setup_postgrest(Some(JWT_SECRET), None).await?;

        // Insert test data
        let db_url = postgres_url(ctx.postgres_port);
        insert_test_data(&db_url, 3).await?;

        // Create a JWT token with authenticated role
        // Header: {"alg":"HS256","typ":"JWT"}
        // Payload: {"role":"authenticated"}
        // This is a valid JWT for testing purposes
        let jwt_token = create_test_jwt("authenticated", JWT_SECRET);

        let client = reqwest::Client::new();

        // Request with JWT should use authenticated role
        let response = client
            .get(format!("{}/todos", postgrest_url(ctx.postgrest_port)))
            .header("Authorization", format!("Bearer {}", jwt_token))
            .send()
            .await?;

        assert!(
            response.status().is_success(),
            "JWT authenticated request should succeed: {}",
            response.status()
        );

        let body: Vec<serde_json::Value> = response.json().await?;
        assert!(
            body.len() >= 3,
            "Should be able to read todos as authenticated user"
        );

        Ok(())
    }

    /// Test that max_rows configuration limits responses
    #[tokio::test]
    async fn test_max_rows_limit() -> Result<()> {
        let ctx = setup_postgrest(None, Some(10)).await?;

        // Insert more rows than the limit
        let db_url = postgres_url(ctx.postgres_port);
        insert_test_data(&db_url, 20).await?;

        let client = reqwest::Client::new();
        let response = client
            .get(format!("{}/todos", postgrest_url(ctx.postgrest_port)))
            .send()
            .await?;

        assert!(
            response.status().is_success(),
            "Request should succeed: {}",
            response.status()
        );

        let body: Vec<serde_json::Value> = response.json().await?;
        assert_eq!(
            body.len(),
            10,
            "Should return exactly max_rows (10) rows, got {}",
            body.len()
        );

        Ok(())
    }

    /// Creates a simple JWT token for testing
    /// Note: In production, use a proper JWT library
    fn create_test_jwt(role: &str, secret: &str) -> String {
        // Base64URL encode without padding
        fn base64url_encode(data: &[u8]) -> String {
            let mut buf = String::new();
            for chunk in data.chunks(3) {
                let mut n = (chunk[0] as u32) << 16;
                if chunk.len() > 1 {
                    n |= (chunk[1] as u32) << 8;
                }
                if chunk.len() > 2 {
                    n |= chunk[2] as u32;
                }

                let chars: Vec<char> =
                    "ABCDEFGHIJKLMNOPQRSTUVWXYZabcdefghijklmnopqrstuvwxyz0123456789-_"
                        .chars()
                        .collect();

                buf.push(chars[(n >> 18 & 0x3F) as usize]);
                buf.push(chars[(n >> 12 & 0x3F) as usize]);
                if chunk.len() > 1 {
                    buf.push(chars[(n >> 6 & 0x3F) as usize]);
                }
                if chunk.len() > 2 {
                    buf.push(chars[(n & 0x3F) as usize]);
                }
            }
            buf
        }

        // HMAC-SHA256 implementation (simplified for testing)
        fn hmac_sha256(key: &[u8], message: &[u8]) -> [u8; 32] {
            const BLOCK_SIZE: usize = 64;
            const OPAD: u8 = 0x5c;
            const IPAD: u8 = 0x36;

            // Prepare key
            let mut k = [0u8; BLOCK_SIZE];
            if key.len() > BLOCK_SIZE {
                let hash = sha256(key);
                k[..32].copy_from_slice(&hash);
            } else {
                k[..key.len()].copy_from_slice(key);
            }

            // Inner hash
            let mut inner = Vec::with_capacity(BLOCK_SIZE + message.len());
            for i in 0..BLOCK_SIZE {
                inner.push(k[i] ^ IPAD);
            }
            inner.extend_from_slice(message);
            let inner_hash = sha256(&inner);

            // Outer hash
            let mut outer = Vec::with_capacity(BLOCK_SIZE + 32);
            for i in 0..BLOCK_SIZE {
                outer.push(k[i] ^ OPAD);
            }
            outer.extend_from_slice(&inner_hash);
            sha256(&outer)
        }

        // SHA-256 implementation
        fn sha256(data: &[u8]) -> [u8; 32] {
            // Initial hash values
            let mut h: [u32; 8] = [
                0x6a09e667, 0xbb67ae85, 0x3c6ef372, 0xa54ff53a, 0x510e527f, 0x9b05688c, 0x1f83d9ab,
                0x5be0cd19,
            ];

            // Round constants
            let k: [u32; 64] = [
                0x428a2f98, 0x71374491, 0xb5c0fbcf, 0xe9b5dba5, 0x3956c25b, 0x59f111f1, 0x923f82a4,
                0xab1c5ed5, 0xd807aa98, 0x12835b01, 0x243185be, 0x550c7dc3, 0x72be5d74, 0x80deb1fe,
                0x9bdc06a7, 0xc19bf174, 0xe49b69c1, 0xefbe4786, 0x0fc19dc6, 0x240ca1cc, 0x2de92c6f,
                0x4a7484aa, 0x5cb0a9dc, 0x76f988da, 0x983e5152, 0xa831c66d, 0xb00327c8, 0xbf597fc7,
                0xc6e00bf3, 0xd5a79147, 0x06ca6351, 0x14292967, 0x27b70a85, 0x2e1b2138, 0x4d2c6dfc,
                0x53380d13, 0x650a7354, 0x766a0abb, 0x81c2c92e, 0x92722c85, 0xa2bfe8a1, 0xa81a664b,
                0xc24b8b70, 0xc76c51a3, 0xd192e819, 0xd6990624, 0xf40e3585, 0x106aa070, 0x19a4c116,
                0x1e376c08, 0x2748774c, 0x34b0bcb5, 0x391c0cb3, 0x4ed8aa4a, 0x5b9cca4f, 0x682e6ff3,
                0x748f82ee, 0x78a5636f, 0x84c87814, 0x8cc70208, 0x90befffa, 0xa4506ceb, 0xbef9a3f7,
                0xc67178f2,
            ];

            // Pre-processing: adding padding bits
            let bit_len = (data.len() as u64) * 8;
            let mut padded = data.to_vec();
            padded.push(0x80);
            while (padded.len() % 64) != 56 {
                padded.push(0);
            }
            padded.extend_from_slice(&bit_len.to_be_bytes());

            // Process each 512-bit chunk
            for chunk in padded.chunks(64) {
                let mut w = [0u32; 64];
                for (i, word) in chunk.chunks(4).enumerate() {
                    w[i] = u32::from_be_bytes(word.try_into().unwrap());
                }
                for i in 16..64 {
                    let s0 =
                        w[i - 15].rotate_right(7) ^ w[i - 15].rotate_right(18) ^ (w[i - 15] >> 3);
                    let s1 =
                        w[i - 2].rotate_right(17) ^ w[i - 2].rotate_right(19) ^ (w[i - 2] >> 10);
                    w[i] = w[i - 16]
                        .wrapping_add(s0)
                        .wrapping_add(w[i - 7])
                        .wrapping_add(s1);
                }

                let mut a = h[0];
                let mut b = h[1];
                let mut c = h[2];
                let mut d = h[3];
                let mut e = h[4];
                let mut f = h[5];
                let mut g = h[6];
                let mut hh = h[7];

                for i in 0..64 {
                    let s1 = e.rotate_right(6) ^ e.rotate_right(11) ^ e.rotate_right(25);
                    let ch = (e & f) ^ ((!e) & g);
                    let temp1 = hh
                        .wrapping_add(s1)
                        .wrapping_add(ch)
                        .wrapping_add(k[i])
                        .wrapping_add(w[i]);
                    let s0 = a.rotate_right(2) ^ a.rotate_right(13) ^ a.rotate_right(22);
                    let maj = (a & b) ^ (a & c) ^ (b & c);
                    let temp2 = s0.wrapping_add(maj);

                    hh = g;
                    g = f;
                    f = e;
                    e = d.wrapping_add(temp1);
                    d = c;
                    c = b;
                    b = a;
                    a = temp1.wrapping_add(temp2);
                }

                h[0] = h[0].wrapping_add(a);
                h[1] = h[1].wrapping_add(b);
                h[2] = h[2].wrapping_add(c);
                h[3] = h[3].wrapping_add(d);
                h[4] = h[4].wrapping_add(e);
                h[5] = h[5].wrapping_add(f);
                h[6] = h[6].wrapping_add(g);
                h[7] = h[7].wrapping_add(hh);
            }

            let mut result = [0u8; 32];
            for (i, &val) in h.iter().enumerate() {
                result[i * 4..(i + 1) * 4].copy_from_slice(&val.to_be_bytes());
            }
            result
        }

        let header = r#"{"alg":"HS256","typ":"JWT"}"#;
        let payload = format!(r#"{{"role":"{}"}}"#, role);

        let header_b64 = base64url_encode(header.as_bytes());
        let payload_b64 = base64url_encode(payload.as_bytes());
        let message = format!("{}.{}", header_b64, payload_b64);

        let signature = hmac_sha256(secret.as_bytes(), message.as_bytes());
        let signature_b64 = base64url_encode(&signature);

        format!("{}.{}", message, signature_b64)
    }
}
