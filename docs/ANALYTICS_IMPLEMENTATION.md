# Analytics Feature Implementation Plan

This document tracks the implementation of the `analytics` feature for the Supabase testcontainers crate.

## Overview

Supabase Analytics is powered by **Logflare**, a centralized logging and analytics platform. Key characteristics:

- Logflare is an Elixir/Phoenix application for log ingestion and querying
- Supports PostgreSQL or BigQuery as storage backends
- Single-tenant mode designed for self-hosted deployments
- Integrates with Supabase services via Vector for log routing
- Provides a `/health` endpoint for readiness checks

**Docker Image:** `supabase/logflare`
**Default Port:** 4000
**Health Check:** `curl http://localhost:4000/health`

## Architecture Notes

In Supabase's architecture:
1. `supabase/logflare` - Analytics server for log ingestion and querying
2. Vector - Routes logs from all services to Logflare
3. PostgreSQL - Stores analytics data (in `_analytics` schema)
4. Kong - Routes `/analytics/v1` to Logflare

For testing, users will typically:
1. Start PostgreSQL with the analytics schema
2. Start the Analytics container with database connection
3. Query logs via the Logflare API or health endpoint

## Implementation Sections

### Section 1: Core Struct Definition
- [x] Change `env_vars` from `HashMap` to `BTreeMap`
- [x] Add `tag: String` field for version customization
- [x] Define `ANALYTICS_PORT` constant (4000)
- [x] Update module constants (NAME to "supabase/logflare", TAG to "1.26.13")

### Section 2: Default Implementation
- [x] Configure HTTP port (4000)
- [x] Set LOGFLARE_SINGLE_TENANT default (true)
- [x] Set LOGFLARE_SUPABASE_MODE default (true)
- [x] Configure default node host (127.0.0.1)
- [x] Set default database schema (_analytics)

### Section 3: Builder Methods
- [x] `with_postgres_backend_url` - PostgreSQL connection URL for backend storage
- [x] `with_postgres_backend_schema` - Database schema for analytics
- [x] `with_db_hostname` - Database hostname
- [x] `with_db_port` - Database port
- [x] `with_db_username` - Database username
- [x] `with_db_password` - Database password
- [x] `with_db_database` - Database name
- [x] `with_db_schema` - Database schema (alternative to backend schema)
- [x] `with_public_access_token` - Public API token for ingestion/querying
- [x] `with_private_access_token` - Private API token for management
- [x] `with_encryption_key` - Base64 encryption key for sensitive data
- [x] `with_node_host` - Logflare node host
- [x] `with_single_tenant` - Enable/disable single tenant mode
- [x] `with_supabase_mode` - Enable/disable Supabase mode
- [x] `with_feature_flag_override` - Feature flag overrides
- [x] `with_log_level` - Log verbosity (error, warning, info)
- [x] `with_http_port` - Phoenix HTTP port
- [x] `with_tag` - Custom Docker image version
- [x] `with_env` - Generic environment variable

### Section 4: Image Trait Implementation
- [x] Implement `name()` returning "supabase/logflare"
- [x] Implement `tag()` returning custom tag
- [x] Implement `ready_conditions()` with health check message
- [x] Implement `expose_ports()` returning ANALYTICS_PORT (4000)
- [x] Implement `env_vars()` returning BTreeMap iterator

### Section 5: Library Exports
- [x] Update `lib.rs` to export `ANALYTICS_PORT` with `Analytics`

### Section 6: Module Documentation
- [x] Add comprehensive module-level rustdoc
- [x] Include usage examples
- [x] Document all configuration options
- [x] Add struct-level documentation
- [x] Note PostgreSQL backend requirement

### Section 7: Unit Tests
- [x] Test default configuration values
- [x] Test `name()` returns correct image
- [x] Test `tag()` returns correct version
- [x] Test `ANALYTICS_PORT` constant
- [x] Test each builder method
- [x] Test builder method chaining
- [x] Test `expose_ports()` returns correct port
- [x] Test `ready_conditions()` is non-empty

### Section 8: Validation
- [x] Run `cargo fmt`
- [x] Run `cargo clippy --all-features`
- [x] Run `cargo test --features analytics`
- [x] Run `cargo build --features analytics`

## Environment Variables Reference

Based on Supabase docker-compose and Logflare documentation:

| Variable | Description | Default |
|----------|-------------|---------|
| `LOGFLARE_NODE_HOST` | Erlang node host | 127.0.0.1 |
| `DB_USERNAME` | Database username | supabase_admin |
| `DB_DATABASE` | Database name | _supabase |
| `DB_HOSTNAME` | Database hostname | - |
| `DB_PORT` | Database port | 5432 |
| `DB_PASSWORD` | Database password | - |
| `DB_SCHEMA` | Database schema | _analytics |
| `LOGFLARE_PUBLIC_ACCESS_TOKEN` | Public API token | - |
| `LOGFLARE_PRIVATE_ACCESS_TOKEN` | Private API token | - |
| `LOGFLARE_SINGLE_TENANT` | Single tenant mode | true |
| `LOGFLARE_SUPABASE_MODE` | Supabase integration mode | true |
| `POSTGRES_BACKEND_URL` | Full PostgreSQL URL for backend | - |
| `POSTGRES_BACKEND_SCHEMA` | Backend schema | _analytics |
| `LOGFLARE_FEATURE_FLAG_OVERRIDE` | Feature flags | multibackend=true |
| `LOGFLARE_DB_ENCRYPTION_KEY` | Base64 encryption key | - |
| `LOGFLARE_LOG_LEVEL` | Log level | warn |
| `PHX_HTTP_PORT` | Phoenix HTTP port | 4000 |

## Usage Pattern

```rust
use supabase_testcontainers_modules::{Analytics, ANALYTICS_PORT};
use testcontainers::runners::AsyncRunner;

// Start PostgreSQL first (with _analytics schema)
// ...

// Start Analytics with PostgreSQL backend
let analytics = Analytics::default()
    .with_postgres_backend_url("postgresql://supabase_admin:password@db:5432/_supabase")
    .with_postgres_backend_schema("_analytics")
    .with_public_access_token("your-public-token")
    .with_private_access_token("your-private-token")
    .start()
    .await?;

let port = analytics.get_host_port_ipv4(ANALYTICS_PORT).await?;

// Verify health
let resp = reqwest::get(format!("http://localhost:{}/health", port)).await?;
assert_eq!(resp.status(), 200);
```

## Commit History

| Section | Commit | Status |
|---------|--------|--------|
| Section 1-4, 6-7 | - | Complete |
| Section 5 | - | Complete |
| Section 8 | - | Complete (validated) |

## Notes

- Logflare is an Elixir/Phoenix application (hence PHX_* env vars)
- Single-tenant mode is required for self-hosted deployments
- The PostgreSQL backend is recommended for testing/development
- BigQuery backend is available for production workloads
- The `_analytics` schema is used to isolate analytics data
- Encryption key is recommended for production (sensitive data at rest)

## Sources

- [Supabase Self-Hosting Analytics Introduction](https://supabase.com/docs/reference/self-hosting-analytics/introduction)
- [Supabase Analytics Config Docs](https://supabase.com/docs/guides/self-hosting/analytics/config)
- [Logflare Self-Hosting Docs](https://docs.logflare.app/self-hosting/)
- [Supabase Docker Compose](https://github.com/supabase/supabase/blob/master/docker/docker-compose.yml)
- [supabase/logflare Docker Hub](https://hub.docker.com/r/supabase/logflare)
