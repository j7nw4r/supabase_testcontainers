//! Integration tests for Supabase Storage container setup patterns
//!
//! These tests verify the Storage container configuration and basic connectivity.
//!
//! Note: Full storage functionality (bucket creation, file operations) requires
//! the complete Supabase PostgreSQL schema including storage schema migrations.
//! The storage-api container runs its own migrations but requires specific
//! PostgreSQL roles (anon, authenticated, service_role) to be pre-created.
//!
//! These tests verify:
//! - Container startup with proper configuration
//! - Health endpoint accessibility
//! - File size limit configuration
//!
//! Run with: `cargo test --features storage,const --test storage_integration`

use anyhow::Result;
use std::sync::atomic::{AtomicU64, Ordering};
use std::time::Duration;
use supabase_testcontainers_modules::{Storage, LOCAL_HOST, STORAGE_PORT};
use testcontainers::runners::AsyncRunner;
use testcontainers::ImageExt;
use testcontainers_modules::postgres::Postgres;
use tokio_postgres::NoTls;

/// PostgreSQL port constant
const POSTGRES_PORT: u16 = 5432;
/// Network name for container-to-container communication
const TEST_NETWORK: &str = "storage-test-network";
/// PostgreSQL container alias on the shared network
const POSTGRES_ALIAS: &str = "storage-db";

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

/// Returns the base URL for Storage API
fn storage_url(port: u16) -> String {
    format!("http://{}:{}", LOCAL_HOST, port)
}

#[cfg(test)]
mod tests {
    use super::*;
    use testcontainers::ContainerAsync;

    /// Helper struct for PostgreSQL + Storage setup
    struct StorageContext {
        #[allow(dead_code)]
        postgres: ContainerAsync<Postgres>,
        #[allow(dead_code)]
        storage: ContainerAsync<Storage>,
        #[allow(dead_code)]
        postgres_port: u16,
        storage_port: u16,
        #[allow(dead_code)]
        service_key: String,
        #[allow(dead_code)]
        anon_key: String,
    }

    /// Sets up PostgreSQL and Storage for testing
    async fn setup_storage(file_size_limit: Option<u64>) -> Result<StorageContext> {
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

        // Set up storage schema
        let db_url = postgres_url(postgres_port);
        setup_storage_schema(&db_url).await?;

        // Create JWT tokens for authentication
        let service_key = create_test_jwt("service_role", JWT_SECRET);
        let anon_key = create_test_jwt("anon", JWT_SECRET);

        // Connection string for Storage (uses container name on shared network)
        let storage_db_url = format!(
            "postgres://postgres:postgres@{}:{}/postgres",
            postgres_name, POSTGRES_PORT
        );

        // Build Storage with configuration
        let mut storage_image = Storage::default()
            .with_database_url(&storage_db_url)
            .with_jwt_secret(JWT_SECRET)
            .with_anon_key(&anon_key)
            .with_service_key(&service_key);

        if let Some(limit) = file_size_limit {
            storage_image = storage_image.with_file_size_limit(limit);
        }

        let storage = storage_image
            .with_startup_timeout(Duration::from_secs(60))
            .with_network(&network_name)
            .start()
            .await?;
        let storage_port = storage.get_host_port_ipv4(STORAGE_PORT).await?;

        // Wait for Storage to connect to database
        tokio::time::sleep(Duration::from_secs(3)).await;

        Ok(StorageContext {
            postgres,
            storage,
            postgres_port,
            storage_port,
            service_key,
            anon_key,
        })
    }

    /// Sets up the PostgreSQL roles and extensions that storage-api expects
    async fn setup_storage_schema(db_url: &str) -> Result<()> {
        let (client, connection) = tokio_postgres::connect(db_url, NoTls).await?;

        tokio::spawn(async move {
            if let Err(e) = connection.await {
                eprintln!("connection error: {}", e);
            }
        });

        // Create roles and extensions that storage-api expects
        // Storage-api will run its own migrations to create tables
        client
            .batch_execute(
                r#"
            -- Create roles that storage-api expects
            DO $$
            BEGIN
                IF NOT EXISTS (SELECT FROM pg_roles WHERE rolname = 'anon') THEN
                    CREATE ROLE anon NOLOGIN;
                END IF;
                IF NOT EXISTS (SELECT FROM pg_roles WHERE rolname = 'authenticated') THEN
                    CREATE ROLE authenticated NOLOGIN;
                END IF;
                IF NOT EXISTS (SELECT FROM pg_roles WHERE rolname = 'service_role') THEN
                    CREATE ROLE service_role NOLOGIN;
                END IF;
                IF NOT EXISTS (SELECT FROM pg_roles WHERE rolname = 'supabase_storage_admin') THEN
                    CREATE ROLE supabase_storage_admin NOLOGIN;
                END IF;
            END
            $$;

            -- Create extensions that storage-api needs
            CREATE EXTENSION IF NOT EXISTS "uuid-ossp";
            "#,
            )
            .await?;

        Ok(())
    }

    /// Test that PostgreSQL and Storage containers start successfully
    #[tokio::test]
    async fn test_containers_start() -> Result<()> {
        let ctx = setup_storage(None).await?;

        assert!(ctx.postgres_port > 0, "PostgreSQL port should be assigned");
        assert!(ctx.storage_port > 0, "Storage port should be assigned");

        Ok(())
    }

    /// Test that Storage health endpoint returns 200
    #[tokio::test]
    async fn test_health_endpoint_returns_200() -> Result<()> {
        let ctx = setup_storage(None).await?;

        let client = reqwest::Client::new();
        let response = client
            .get(format!("{}/status", storage_url(ctx.storage_port)))
            .send()
            .await?;

        assert!(
            response.status().is_success(),
            "Expected success, got: {}",
            response.status()
        );

        Ok(())
    }

    /// Test that file size limit configuration is applied
    /// Note: This test verifies the configuration is accepted; full enforcement
    /// requires complete PostgreSQL schema setup
    #[tokio::test]
    async fn test_file_size_limit_configuration() -> Result<()> {
        // Set up storage with 100 byte limit
        let ctx = setup_storage(Some(100)).await?;

        // Verify container started with the configuration
        assert!(ctx.storage_port > 0, "Storage port should be assigned");

        // Verify health endpoint still works with custom config
        let client = reqwest::Client::new();
        let response = client
            .get(format!("{}/status", storage_url(ctx.storage_port)))
            .send()
            .await?;

        assert!(
            response.status().is_success(),
            "Expected success with custom file size limit, got: {}",
            response.status()
        );

        Ok(())
    }

    /// Creates a simple JWT token for testing
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

        // HMAC-SHA256 implementation
        fn hmac_sha256(key: &[u8], message: &[u8]) -> [u8; 32] {
            const BLOCK_SIZE: usize = 64;
            const OPAD: u8 = 0x5c;
            const IPAD: u8 = 0x36;

            let mut k = [0u8; BLOCK_SIZE];
            if key.len() > BLOCK_SIZE {
                let hash = sha256(key);
                k[..32].copy_from_slice(&hash);
            } else {
                k[..key.len()].copy_from_slice(key);
            }

            let mut inner = Vec::with_capacity(BLOCK_SIZE + message.len());
            for i in 0..BLOCK_SIZE {
                inner.push(k[i] ^ IPAD);
            }
            inner.extend_from_slice(message);
            let inner_hash = sha256(&inner);

            let mut outer = Vec::with_capacity(BLOCK_SIZE + 32);
            for i in 0..BLOCK_SIZE {
                outer.push(k[i] ^ OPAD);
            }
            outer.extend_from_slice(&inner_hash);
            sha256(&outer)
        }

        // SHA-256 implementation
        fn sha256(data: &[u8]) -> [u8; 32] {
            let mut h: [u32; 8] = [
                0x6a09e667, 0xbb67ae85, 0x3c6ef372, 0xa54ff53a, 0x510e527f, 0x9b05688c, 0x1f83d9ab,
                0x5be0cd19,
            ];

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

            let bit_len = (data.len() as u64) * 8;
            let mut padded = data.to_vec();
            padded.push(0x80);
            while (padded.len() % 64) != 56 {
                padded.push(0);
            }
            padded.extend_from_slice(&bit_len.to_be_bytes());

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
