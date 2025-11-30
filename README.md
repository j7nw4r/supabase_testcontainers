# supabase-testcontainers-modules

[![Crates.io](https://img.shields.io/crates/v/supabase-testcontainers-modules.svg)](https://crates.io/crates/supabase-testcontainers-modules)
[![Docs.rs](https://docs.rs/supabase-testcontainers-modules/badge.svg)](https://docs.rs/supabase-testcontainers-modules)

> Note: This project was created with assistance from AI tools.

Testcontainers modules for Supabase services in Rust. Run real Supabase containers in your integration tests.

## Installation

```toml
[dependencies]
supabase-testcontainers-modules = { version = "1.0.0", features = ["auth"] }

[dev-dependencies]
testcontainers = "0.23"
testcontainers-modules = { version = "0.11", features = ["postgres"] }
tokio = { version = "1", features = ["full"] }
```

## Quick Start

```rust
use supabase_testcontainers_modules::{Auth, AUTH_PORT, LOCAL_HOST};
use testcontainers::{runners::AsyncRunner, ImageExt};
use testcontainers_modules::postgres::Postgres;

#[tokio::test]
async fn test_auth() -> anyhow::Result<()> {
    // Use shared network for container-to-container communication
    let network = "test-network";
    let pg_container = "postgres-db";

    // Start PostgreSQL 15 on shared network
    let postgres = Postgres::default()
        .with_tag("15-alpine")
        .with_network(network)
        .with_container_name(pg_container)
        .start()
        .await?;
    let pg_port = postgres.get_host_port_ipv4(5432).await?;

    // Auth connects via container name, schema init via localhost
    let auth_db_url = format!(
        "postgres://supabase_auth_admin:password@{}:5432/postgres",
        pg_container
    );
    let local_db_url = format!(
        "postgres://postgres:postgres@{}:{}/postgres",
        LOCAL_HOST, pg_port
    );

    // Initialize schema and start Auth on same network
    let auth = Auth::default()
        .with_db_url(&auth_db_url)
        .init_db_schema(&local_db_url, "password")
        .await?
        .with_network(network)
        .start()
        .await?;

    let auth_port = auth.get_host_port_ipv4(AUTH_PORT).await?;

    // Verify health
    let resp = reqwest::get(format!("http://localhost:{}/health", auth_port)).await?;
    assert_eq!(resp.status(), 200);

    Ok(())
}
```

## Auth Configuration

```rust
Auth::default()
    .with_db_url("postgres://...")           // PostgreSQL connection (required)
    .with_jwt_secret("32-char-minimum...")   // JWT signing secret
    .with_jwt_expiry(3600)                   // Token expiry in seconds
    .with_site_url("http://localhost:3000")  // Frontend URL
    .with_signup_disabled(false)             // Allow user signups
    .with_anonymous_users(true)              // Enable anonymous auth
    .with_mailer_autoconfirm(true)           // Skip email verification
    .with_sms_autoconfirm(true)              // Skip SMS verification
    .with_log_level("debug")                 // Log verbosity
    .with_tag("v2.183.0")                    // Image version
    .with_env("KEY", "value")                // Custom environment variable
```

## PostgREST Configuration

```rust
PostgREST::default()
    .with_postgres_connection("postgres://...")  // PostgreSQL connection (required)
    .with_db_schemas("public,api")               // Exposed schemas
    .with_db_anon_role("anon")                   // Anonymous role
    .with_jwt_secret("32-char-minimum...")       // JWT validation secret
    .with_jwt_role_claim_key(".role")            // JWT role claim path
    .with_openapi_mode("follow-privileges")      // OpenAPI generation mode
    .with_max_rows(1000)                         // Max rows per response
    .with_pre_request("auth.check")              // Pre-request function
    .with_log_level("warn")                      // Log verbosity
    .with_tag("v12.2.3")                         // Image version
    .with_env("KEY", "value")                    // Custom environment variable
```

## Storage Configuration

```rust
Storage::default()
    .with_database_url("postgres://...")         // PostgreSQL connection (required)
    .with_anon_key("anon-jwt-token")             // Anonymous JWT for public access
    .with_service_key("service-role-jwt")        // Service role JWT (bypasses RLS)
    .with_jwt_secret("32-char-minimum...")       // JWT validation secret
    .with_postgrest_url("http://postgrest:3000") // PostgREST server URL
    .with_storage_backend("file")                // Backend type ("file" or "s3")
    .with_file_storage_path("/var/lib/storage")  // Local storage path
    .with_file_size_limit(52428800)              // Max file size (50MB)
    .with_region("us-east-1")                    // S3 region
    .with_global_s3_bucket("my-bucket")          // S3 bucket name
    .with_tenant_id("default")                   // Tenant identifier
    .with_multitenant(false)                     // Multi-tenant mode
    .with_tag("v1.11.1")                         // Image version
    .with_env("KEY", "value")                    // Custom environment variable
```

## Realtime Configuration

```rust
Realtime::default()
    .with_postgres_connection("postgres://...")     // PostgreSQL connection (required)
    .with_jwt_secret("32-char-minimum...")          // JWT signing secret
    .with_slot_name("realtime_rls")                 // Replication slot name
    .with_temporary_slot(true)                      // Use temporary slot (recommended for tests)
    .with_secure_channels(true)                     // Require JWT for channel subscriptions
    .with_region("local")                           // AWS region
    .with_tenant_id("realtime-dev")                 // Tenant identifier
    .with_secret_key_base("64-char-minimum...")     // Phoenix secret key
    .with_db_host("localhost")                      // Individual DB config (alternative to connection string)
    .with_db_port(5432)
    .with_db_name("postgres")
    .with_db_user("postgres")
    .with_db_password("password")
    .with_db_ssl(false)                             // SSL for DB connection
    .with_tag("v2.33.58")                           // Image version
    .with_env("KEY", "value")                       // Custom environment variable
```

## Functions Configuration

```rust
Functions::default()
    .with_jwt_secret("32-char-minimum...")          // JWT signing secret
    .with_supabase_url("http://kong:8000")          // Supabase API URL
    .with_anon_key("anon-jwt-token")                // Anonymous JWT key
    .with_service_role_key("service-role-jwt")      // Service role JWT (bypasses RLS)
    .with_db_url("postgres://...")                  // PostgreSQL connection string
    .with_verify_jwt(false)                         // Disable JWT verification (for testing)
    .with_main_service_path("/home/deno/functions") // Functions directory path
    .with_worker_timeout_ms(30000)                  // Worker timeout in ms
    .with_max_parallelism(4)                        // Max concurrent workers
    .with_port(9000)                                // Server port
    .with_tag("v1.67.4")                            // Image version
    .with_env("KEY", "value")                       // Custom environment variable
```

## GraphQL Configuration

GraphQL is provided via the pg_graphql PostgreSQL extension (pre-installed in `supabase/postgres`).
To query GraphQL over HTTP, use PostgREST alongside this container.

```rust
// 1. Start PostgreSQL with pg_graphql
GraphQL::default()
    .with_database("postgres")                      // Database name
    .with_user("postgres")                          // PostgreSQL user
    .with_password("postgres")                      // PostgreSQL password
    .with_host("0.0.0.0")                           // Bind address
    .with_port(5432)                                // PostgreSQL port
    .with_jwt_secret("32-char-minimum...")          // JWT secret for pg_graphql
    .with_tag("15.8.1.085")                         // Image version
    .with_env("KEY", "value")                       // Custom environment variable

// 2. Start PostgREST pointing to this database with graphql_public schema
// 3. Query via: POST /rpc/graphql with { "query": "{ ... }" }
```

## Analytics Configuration

Analytics is powered by Logflare, a centralized logging and analytics platform.
It requires a PostgreSQL database for storing analytics data.

```rust
Analytics::default()
    .with_postgres_backend_url("postgresql://...")    // PostgreSQL backend URL (required)
    .with_postgres_backend_schema("_analytics")       // Backend schema
    .with_db_hostname("localhost")                    // Database hostname
    .with_db_port(5432)                               // Database port
    .with_db_username("supabase_admin")               // Database username
    .with_db_password("password")                     // Database password
    .with_db_database("_supabase")                    // Database name
    .with_public_access_token("your-public-token")   // Public API token
    .with_private_access_token("your-private-token") // Private API token
    .with_encryption_key("base64-key...")             // Encryption key for sensitive data
    .with_single_tenant(true)                         // Single-tenant mode
    .with_supabase_mode(true)                         // Supabase integration mode
    .with_log_level("warn")                           // Log verbosity
    .with_tag("1.26.13")                              // Image version
    .with_env("KEY", "value")                         // Custom environment variable
```

## Features

| Feature | Description |
|---------|-------------|
| `auth` | Supabase Auth (GoTrue) container |
| `postgrest` | PostgREST container |
| `storage` | Supabase Storage container |
| `realtime` | Realtime container |
| `functions` | Edge Functions container |
| `graphql` | PostgreSQL with pg_graphql extension |
| `analytics` | Analytics container |

## Requirements

- Docker
- PostgreSQL 12+ (for Auth migrations)

## Implementation Status

| Service | Status |
|---------|--------|
| Auth | Complete |
| PostgREST | Complete |
| Storage | Complete |
| Realtime | Complete |
| Functions | Complete |
| GraphQL | Complete |
| Analytics | Complete |

## License

MIT
