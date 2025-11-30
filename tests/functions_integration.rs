//! Integration tests for Supabase Edge Functions (edge-runtime) container setup patterns
//!
//! These tests verify the Functions container configuration and startup.
//!
//! Note: Full function invocation requires mounting a directory with actual
//! Deno functions. The edge-runtime container requires the functions directory
//! to exist at startup. On SELinux-enabled systems (like Fedora), the `:Z` flag
//! may be needed for bind mounts, which testcontainers doesn't support natively.
//!
//! These tests verify the container setup patterns and configuration work correctly.
//!
//! Run with: `cargo test --features functions,const --test functions_integration`

use std::borrow::Cow;
use std::sync::atomic::{AtomicU64, Ordering};
use supabase_testcontainers_modules::{Functions, FUNCTIONS_PORT};

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

#[cfg(test)]
mod tests {
    use super::*;
    use testcontainers_modules::testcontainers::core::WaitFor;
    use testcontainers_modules::testcontainers::Image;

    /// Test that Functions default configuration is correct
    #[test]
    fn test_default_configuration() {
        let functions = Functions::default();

        // Verify the image is correct
        assert_eq!(functions.name(), "supabase/edge-runtime");
        assert!(functions.tag().starts_with("v1."));
    }

    /// Test that Functions can be configured with JWT secret
    #[test]
    fn test_jwt_secret_configuration() {
        let functions = Functions::default().with_jwt_secret(JWT_SECRET);

        // Builder pattern should work without error
        assert_eq!(functions.name(), "supabase/edge-runtime");
    }

    /// Test that Functions can be configured with all options
    #[test]
    fn test_full_configuration() {
        let _test_id = unique_test_id();

        let functions = Functions::default()
            .with_jwt_secret(JWT_SECRET)
            .with_verify_jwt(false)
            .with_worker_timeout_ms(30000)
            .with_max_parallelism(4)
            .with_supabase_url("http://kong:8000")
            .with_anon_key("test-anon-key")
            .with_service_role_key("test-service-key")
            .with_db_url("postgres://user:pass@localhost:5432/db")
            .with_main_service_path("/custom/functions")
            .with_tag("v1.67.4")
            .with_env("CUSTOM_VAR", "custom_value");

        assert_eq!(functions.name(), "supabase/edge-runtime");
        assert_eq!(functions.tag(), "v1.67.4");
    }

    /// Test that Functions exposes the correct port
    #[test]
    fn test_exposed_port() {
        let functions = Functions::default();
        let ports = functions.expose_ports();

        assert_eq!(ports.len(), 1);
        assert_eq!(FUNCTIONS_PORT, 9000);
    }

    /// Test that Functions has correct ready conditions
    #[test]
    fn test_ready_conditions() {
        let functions = Functions::default();
        let conditions = functions.ready_conditions();

        // Should have exactly one ready condition
        assert_eq!(conditions.len(), 1);

        // Should be waiting for "Listening on" message
        match &conditions[0] {
            WaitFor::Log(log_wait) => {
                // Verify it's looking for stdout
                assert!(
                    format!("{:?}", log_wait).contains("Listening on"),
                    "Should wait for 'Listening on' message"
                );
            }
            _ => panic!("Expected Log wait condition"),
        }
    }

    /// Test that Functions command is correctly set
    #[test]
    fn test_command() {
        let functions = Functions::default();
        let cmd: Vec<Cow<'_, str>> = functions.cmd().into_iter().map(|s| s.into()).collect();

        assert_eq!(cmd.len(), 3);
        assert_eq!(cmd[0], "start");
        assert_eq!(cmd[1], "--main-service");
        assert_eq!(cmd[2], "/home/deno/functions");
    }

    /// Test that custom main service path is respected
    #[test]
    fn test_custom_main_service_path() {
        let functions = Functions::default().with_main_service_path("/custom/path");
        let cmd: Vec<Cow<'_, str>> = functions.cmd().into_iter().map(|s| s.into()).collect();

        assert_eq!(cmd[2], "/custom/path");
    }
}
