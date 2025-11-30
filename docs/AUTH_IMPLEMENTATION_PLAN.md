# Supabase Auth Testcontainer Implementation Plan

This document outlines the implementation plan for building a production-ready Supabase Auth testcontainer module following patterns from [testcontainers-rs-modules-community](https://github.com/testcontainers/testcontainers-rs-modules-community).

## Overview

- **Docker Image:** `supabase/gotrue` (v2.183.0)
- **Port:** 9999
- **Health Check:** `GET /health` returns 200
- **Database:** Requires PostgreSQL with `auth` schema

---

## Implementation Steps

### 1. Core Image Configuration Fixes

- [x] 1.1. Update image name from `supabase/auth` to `supabase/gotrue`
- [x] 1.2. Update image tag from `v2.164.0` to `v2.183.0`
- [x] 1.3. Add `AUTH_PORT` constant (9999)
- [x] 1.4. Export `AUTH_PORT` from `lib.rs`
- [x] 1.5. Remove `cmd()` override (use default entrypoint)
- [x] 1.6. Implement proper `ready_conditions()` with log-based wait for "API started"

### 2. Struct Refactoring

- [x] 2.1. Change `env_vars` from `HashMap` to `BTreeMap` for consistent ordering
- [x] 2.2. Add `tag: String` field to allow custom version override
- [x] 2.3. Update `Image::tag()` to return `&self.tag` instead of constant
- [x] 2.4. Update `Default` impl for new fields

### 3. Builder Methods

- [x] 3.1. Refactor `new()` to use builder pattern (return `Self::default().with_db_url(...)`)
- [x] 3.2. Add `with_db_url(impl Into<String>)` method
- [x] 3.3. Add `with_jwt_secret(impl Into<String>)` method
- [x] 3.4. Add `with_jwt_expiry(u32)` method
- [x] 3.5. Add `with_api_external_url(impl Into<String>)` method
- [x] 3.6. Add `with_site_url(impl Into<String>)` method
- [x] 3.7. Add `with_signup_disabled(bool)` method
- [x] 3.8. Add `with_anonymous_users(bool)` method
- [x] 3.9. Add `with_mailer_autoconfirm(bool)` method
- [x] 3.10. Add `with_sms_autoconfirm(bool)` method
- [x] 3.11. Add `with_log_level(impl Into<String>)` method
- [x] 3.12. Add `with_tag(impl Into<String>)` method
- [x] 3.13. Add `with_env(impl Into<String>, impl Into<String>)` for custom env vars
- [x] 3.14. Remove `new_with_env()` (replaced by `with_env()` chain)

### 4. Default Configuration Cleanup

- [x] 4.1. Remove unnecessary default env vars (GitHub OAuth placeholders, etc.)
- [x] 4.2. Keep only essential defaults (DB driver, API host, port, namespace)
- [x] 4.3. Set sensible test defaults (autoconfirm enabled, debug logging)
- [x] 4.4. Use a proper length JWT secret (min 32 chars)

### 5. Database Initialization Improvements

- [x] 5.1. Add `auth_admin_password` parameter to `init_db_schema()`
- [x] 5.2. Improve error messages with context
- [x] 5.3. Make `DB_NAMESPACE` have a default fallback ("auth")
- [x] 5.4. Remove `println!` debug statement

### 6. Dependencies Update

- [x] 6.1. Verify testcontainers supports HTTP wait strategy (using log-based wait)
- [x] 6.2. Add `reqwest` with `json` feature to dev-dependencies
- [x] 6.3. Add `serde_json` to dev-dependencies
- [x] 6.4. Ensure tokio dev-dependency has `full` and `test-util` features

### 7. Unit Tests

- [x] 7.1. Add test for default configuration values
- [x] 7.2. Add test for `name()` returns correct image name
- [x] 7.3. Add test for `tag()` returns correct version
- [x] 7.4. Add test for builder method chaining
- [x] 7.5. Add test for `with_tag()` overrides default tag
- [x] 7.6. Add test for `with_env()` adds custom environment variable

### 8. Integration Test Setup

- [x] 8.1. Create `tests/auth_integration.rs` file
- [x] 8.2. Create helper function `setup_auth_with_postgres()` that starts both containers
- [x] 8.3. Ensure proper connection string handling (docker internal vs localhost)

### 9. Integration Tests - Basic Functionality

- [x] 9.1. Add test: Auth container starts successfully with PostgreSQL
- [x] 9.2. Add test: Health endpoint returns 200
- [x] 9.3. Add test: Settings endpoint returns configuration

### 10. Integration Tests - Authentication Flows

- [x] 10.1. Add test: Anonymous signup works when enabled
- [x] 10.2. Add test: Email signup with autoconfirm returns tokens
- [x] 10.3. Add test: Signup is rejected when disabled
- [x] 10.4. Add test: Token refresh works
- [x] 10.5. Add test: User retrieval with access token works

### 11. Integration Tests - Database Verification

- [x] 11.1. Add test: Auth schema is created in PostgreSQL
- [x] 11.2. Add test: `supabase_auth_admin` user is created
- [x] 11.3. Add test: Migrations run successfully (verify via health check)

### 12. Documentation

- [x] 12.1. Update module-level doc comment in `auth.rs`
- [x] 12.2. Add doc comments to all public methods
- [x] 12.3. Add usage examples in doc comments
- [x] 12.4. Update README.md with Auth usage example

### 13. Cleanup

- [x] 13.1. Remove commented-out test code in existing `test_auth_default`
- [x] 13.2. Run `cargo fmt`
- [x] 13.3. Run `cargo clippy --all-features` and fix warnings
- [x] 13.4. Run full test suite `cargo test --all-features`

---

## Code Reference

### Target Struct Design

```rust
#[derive(Debug, Clone)]
pub struct Auth {
    env_vars: BTreeMap<String, String>,
    tag: String,
}
```

### Key Constants

```rust
const NAME: &str = "supabase/gotrue";
const TAG: &str = "v2.183.0";
pub const AUTH_PORT: u16 = 9999;
```

### Ready Condition

```rust
fn ready_conditions(&self) -> Vec<WaitFor> {
    vec![WaitFor::message_on_stderr("API started")]
    // Or HTTP health check if supported:
    // vec![WaitFor::http("/health").with_port(AUTH_PORT)]
}
```

### PostgreSQL + Auth Container Setup Pattern

```rust
// 1. Start PostgreSQL testcontainer
let postgres = Postgres::default().with_host_auth().start().await?;
let pg_port = postgres.get_host_port_ipv4(5432).await?;

// 2. Connection strings
let auth_db_url = format!(
    "postgres://supabase_auth_admin:password@host.docker.internal:{}/postgres",
    pg_port
);
let local_db_url = format!(
    "postgres://postgres:postgres@localhost:{}/postgres",
    pg_port
);

// 3. Initialize and start Auth
let auth = Auth::default()
    .with_db_url(&auth_db_url)
    .init_db_schema(&local_db_url, "password")
    .await?
    .start()
    .await?;

let auth_port = auth.get_host_port_ipv4(AUTH_PORT).await?;
```

---

## Dependencies

```toml
[dependencies]
testcontainers = { version = "0.23.1", features = ["default"] }
testcontainers-modules = { version = "0.11.4", features = ["postgres"] }
tokio = { version = "1.24.1" }
tokio-postgres = "0.7.11"
anyhow = "1.0.86"
thiserror = "2.0.11"

[dev-dependencies]
reqwest = { version = "0.12", features = ["json"] }
serde_json = "1.0"
tokio = { version = "1.24.1", features = ["full", "test-util"] }
```

---

## Resources

- [Supabase Auth Repository](https://github.com/supabase/auth)
- [Supabase GoTrue Docker Image](https://hub.docker.com/r/supabase/gotrue)
- [testcontainers-rs-modules-community](https://github.com/testcontainers/testcontainers-rs-modules-community)
- [Supabase Self-Hosting Docker Compose](https://github.com/supabase/supabase/tree/master/docker)
