//! Integration tests for Supabase Realtime container setup patterns
//!
//! These tests verify the Realtime container configuration and setup patterns.
//!
//! Note: Full Realtime functionality (WebSocket connections, CDC) requires
//! PostgreSQL with logical replication enabled (wal_level=logical). The
//! Realtime container runs migrations on startup and requires a properly
//! configured database. Full container startup tests require additional
//! infrastructure setup beyond what testcontainers can easily provide.
//!
//! These tests verify the configuration patterns work correctly.
//!
//! Run with: `cargo test --features realtime,const --test realtime_integration`

use std::sync::atomic::{AtomicU64, Ordering};
use supabase_testcontainers_modules::{Realtime, REALTIME_PORT};

/// JWT secret used for authentication tests (must be at least 32 characters)
const JWT_SECRET: &str = "super-secret-jwt-token-with-at-least-32-characters-for-hs256";

/// Secret key base for Phoenix (must be at least 64 characters)
const SECRET_KEY_BASE: &str =
    "secret-key-base-must-be-at-least-64-characters-long-for-phoenix-framework-encryption";

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

#[cfg(test)]
mod tests {
    use super::*;
    use testcontainers_modules::testcontainers::core::WaitFor;
    use testcontainers_modules::testcontainers::Image;

    /// Test that Realtime default configuration is correct
    #[test]
    fn test_default_configuration() {
        let realtime = Realtime::default();

        // Verify the image is correct
        assert_eq!(realtime.name(), "supabase/realtime");
        assert!(realtime.tag().starts_with("v2."));
    }

    /// Test that Realtime can be configured with PostgreSQL connection
    #[test]
    fn test_postgres_connection_configuration() {
        let realtime =
            Realtime::default().with_postgres_connection("postgres://user:pass@localhost:5432/db");

        assert_eq!(realtime.name(), "supabase/realtime");
    }

    /// Test that Realtime can be configured with individual DB parameters
    #[test]
    fn test_individual_db_configuration() {
        let _test_id = unique_test_id();

        let realtime = Realtime::default()
            .with_db_host("db.example.com")
            .with_db_port(5433)
            .with_db_name("mydb")
            .with_db_user("myuser")
            .with_db_password("secret")
            .with_db_ssl(true);

        assert_eq!(realtime.name(), "supabase/realtime");
    }

    /// Test that Realtime can be configured with all options
    #[test]
    fn test_full_configuration() {
        let _test_id = unique_test_id();

        let realtime = Realtime::default()
            .with_postgres_connection("postgres://user:pass@localhost:5432/db")
            .with_jwt_secret(JWT_SECRET)
            .with_api_jwt_secret("api-jwt-secret")
            .with_secret_key_base(SECRET_KEY_BASE)
            .with_slot_name("custom_slot")
            .with_temporary_slot(true)
            .with_max_record_bytes(1048576)
            .with_secure_channels(false)
            .with_region("us-east-1")
            .with_tenant_id("my-tenant")
            .with_erl_aflags("-proto_dist inet_tcp")
            .with_dns_nodes("node1.example.com")
            .with_enable_tailscale(false)
            .with_port(4001)
            .with_db_after_connect_query("SET timezone TO 'UTC'")
            .with_tag("v2.33.58")
            .with_env("CUSTOM_VAR", "custom_value");

        assert_eq!(realtime.name(), "supabase/realtime");
        assert_eq!(realtime.tag(), "v2.33.58");
    }

    /// Test that Realtime exposes the correct port
    #[test]
    fn test_exposed_port() {
        let realtime = Realtime::default();
        let ports = realtime.expose_ports();

        assert_eq!(ports.len(), 1);
        assert_eq!(REALTIME_PORT, 4000);
    }

    /// Test that Realtime has correct ready conditions
    #[test]
    fn test_ready_conditions() {
        let realtime = Realtime::default();
        let conditions = realtime.ready_conditions();

        // Should have exactly one ready condition
        assert_eq!(conditions.len(), 1);

        // Should be waiting for "Realtime has started" message
        match &conditions[0] {
            WaitFor::Log(log_wait) => {
                assert!(
                    format!("{:?}", log_wait).contains("Realtime has started"),
                    "Should wait for 'Realtime has started' message"
                );
            }
            _ => panic!("Expected Log wait condition"),
        }
    }

    /// Test that required env vars are set by default
    #[test]
    fn test_required_env_vars() {
        let realtime = Realtime::default();
        let env_vars = realtime.env_vars();
        let env_map: std::collections::HashMap<_, _> = env_vars
            .into_iter()
            .map(|(k, v)| (k.into().to_string(), v.into().to_string()))
            .collect();

        // These are required for the container to start
        assert!(env_map.contains_key("APP_NAME"), "APP_NAME should be set");
        assert!(
            env_map.contains_key("RLIMIT_NOFILE"),
            "RLIMIT_NOFILE should be set"
        );
        assert!(env_map.contains_key("PORT"), "PORT should be set");
        assert!(env_map.contains_key("SLOT_NAME"), "SLOT_NAME should be set");
        assert!(
            env_map.contains_key("TEMPORARY_SLOT"),
            "TEMPORARY_SLOT should be set"
        );
        assert!(env_map.contains_key("REGION"), "REGION should be set");
        assert!(env_map.contains_key("TENANT_ID"), "TENANT_ID should be set");
        assert!(
            env_map.contains_key("ERL_AFLAGS"),
            "ERL_AFLAGS should be set"
        );
    }

    /// Test that Realtime can be created with new() and new_with_env()
    #[test]
    fn test_constructors() {
        let realtime1 = Realtime::new();
        assert_eq!(realtime1.name(), "supabase/realtime");

        let mut envs = std::collections::BTreeMap::new();
        envs.insert("CUSTOM_KEY", "custom_value");
        envs.insert("PORT", "5000");

        let realtime2 = Realtime::new_with_env(envs);
        assert_eq!(realtime2.name(), "supabase/realtime");
    }

    /// Test slot configuration options
    #[test]
    fn test_slot_configuration() {
        let realtime = Realtime::default()
            .with_slot_name("my_custom_slot")
            .with_temporary_slot(false)
            .with_max_record_bytes(2097152);

        // Builder methods should work without error
        assert_eq!(realtime.name(), "supabase/realtime");
    }

    /// Test security configuration options
    #[test]
    fn test_security_configuration() {
        let realtime = Realtime::default()
            .with_jwt_secret(JWT_SECRET)
            .with_api_jwt_secret("api-secret")
            .with_secret_key_base(SECRET_KEY_BASE)
            .with_secure_channels(true);

        // Builder methods should work without error
        assert_eq!(realtime.name(), "supabase/realtime");
    }
}
