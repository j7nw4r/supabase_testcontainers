# GraphQL Feature Implementation Plan

This document tracks the implementation of the `graphql` feature for the Supabase testcontainers crate.

## Overview

Supabase uses **pg_graphql**, a PostgreSQL extension that provides GraphQL support directly in the database. Key characteristics:

- **pg_graphql is NOT a standalone container** - it's a PostgreSQL extension
- GraphQL functionality is accessed via PostgREST's RPC endpoint calling `graphql.resolve()`
- The extension is pre-installed in the `supabase/postgres` Docker image
- GraphQL schema is auto-generated from the database schema

**Docker Image:** `supabase/postgres` (includes pg_graphql extension)
**Default Port:** 5432 (PostgreSQL)

## Architecture Notes

In Supabase's architecture:
1. `supabase/postgres` - PostgreSQL with pg_graphql extension
2. PostgREST - Exposes `graphql.resolve()` via HTTP RPC
3. Kong - Routes `/graphql/v1` to PostgREST

For testing, users will typically:
1. Start this GraphQL-enabled PostgreSQL container
2. Start PostgREST pointing to this database
3. Query GraphQL via PostgREST's RPC endpoint

## Implementation Sections

### Section 1: Core Struct Definition
- [x] Change `env_vars` from `HashMap` to `BTreeMap`
- [x] Add `tag: String` field for version customization
- [x] Define `GRAPHQL_PORT` constant (5432 - PostgreSQL port)
- [x] Update module constants (NAME to "supabase/postgres", TAG to specific version)

### Section 2: Default Implementation
- [x] Configure PostgreSQL port (5432)
- [x] Set default database name (postgres)
- [x] Set default user (postgres)
- [x] Set default password (postgres)
- [x] Enable pg_graphql extension loading (pre-installed in supabase/postgres)

### Section 3: Builder Methods
- [x] `with_database` - Database name
- [x] `with_user` - PostgreSQL user
- [x] `with_password` - PostgreSQL password
- [x] `with_host` - Host binding address
- [x] `with_port` - PostgreSQL port
- [x] `with_host_auth_method` - Authentication method
- [x] `with_postgres_args` - PostgreSQL init args
- [x] `with_jwt_secret` - JWT secret for pg_graphql
- [x] `with_tag` - Custom Docker image version
- [x] `with_env` - Generic environment variable
- [x] `connection_string_template` - Helper method for connection strings

### Section 4: Image Trait Implementation
- [x] Implement `name()` returning "supabase/postgres"
- [x] Implement `tag()` returning custom tag
- [x] Implement `ready_conditions()` with PostgreSQL ready message
- [x] Implement `expose_ports()` returning GRAPHQL_PORT (5432)
- [x] Implement `env_vars()` returning BTreeMap iterator
- [x] Implement `exec_after_start()` (empty, for future use)

### Section 5: Library Exports
- [x] Update `lib.rs` to export `GRAPHQL_PORT` with `GraphQL`

### Section 6: Module Documentation
- [x] Add comprehensive module-level rustdoc
- [x] Include usage examples showing PostgREST integration
- [x] Document all configuration options
- [x] Add struct-level documentation
- [x] Note architecture requirements (PostgREST needed for HTTP access)

### Section 7: Unit Tests
- [x] Test default configuration values
- [x] Test `name()` returns correct image
- [x] Test `tag()` returns correct version
- [x] Test `GRAPHQL_PORT` constant
- [x] Test each builder method
- [x] Test builder method chaining
- [x] Test `expose_ports()` returns correct port
- [x] Test `ready_conditions()` is non-empty
- [x] Test `connection_string_template()` helper

### Section 8: Validation
- [x] Run `cargo fmt`
- [x] Run `cargo clippy --all-features`
- [x] Run `cargo test --features graphql`
- [x] Run `cargo build --features graphql`

## Environment Variables Reference

Based on PostgreSQL and pg_graphql configuration:

| Variable | Description | Default |
|----------|-------------|---------|
| `POSTGRES_DB` | Database name | postgres |
| `POSTGRES_USER` | PostgreSQL user | postgres |
| `POSTGRES_PASSWORD` | PostgreSQL password | postgres |
| `POSTGRES_HOST` | Bind address | 0.0.0.0 |
| `POSTGRES_PORT` | PostgreSQL port | 5432 |

## pg_graphql Configuration

pg_graphql is configured via SQL comments on schemas/tables:

```sql
-- Enable name inflection for a schema
COMMENT ON SCHEMA public IS e'@graphql({"inflect_names": true})';

-- Set max rows for a table
COMMENT ON TABLE my_table IS e'@graphql({"max_rows": 100})';
```

## Usage Pattern

```rust
use supabase_testcontainers_modules::{GraphQL, GRAPHQL_PORT, PostgREST, POSTGREST_PORT};
use testcontainers::runners::AsyncRunner;

// 1. Start PostgreSQL with pg_graphql
let graphql_db = GraphQL::default()
    .with_password("secret")
    .start()
    .await?;

let db_port = graphql_db.get_host_port_ipv4(GRAPHQL_PORT).await?;

// 2. Start PostgREST pointing to the database
let db_url = format!("postgres://postgres:secret@host.docker.internal:{}/postgres", db_port);

let postgrest = PostgREST::default()
    .with_postgres_connection(&db_url)
    .with_db_schemas("public,graphql_public")
    .start()
    .await?;

// 3. Query GraphQL via PostgREST RPC
// POST http://localhost:{port}/rpc/graphql
// Body: { "query": "{ ... }" }
```

## Commit History

| Section | Commit | Status |
|---------|--------|--------|
| Section 1-4, 6-7 | - | Complete |
| Section 5 | - | Complete |
| Section 8 | - | Complete (validated) |

## Notes

- pg_graphql is a PostgreSQL extension, not a standalone server
- GraphQL queries are executed via `SELECT graphql.resolve(...)` SQL function
- PostgREST is required to expose GraphQL over HTTP
- The `graphql_public` schema should be added to PostgREST's `PGRST_DB_SCHEMAS`
- The supabase/postgres image has pg_graphql pre-installed and enabled

## Sources

- [pg_graphql Documentation](https://supabase.github.io/pg_graphql/)
- [Supabase Self-Hosting Docker](https://supabase.com/docs/guides/self-hosting/docker)
- [pg_graphql GitHub](https://github.com/supabase/pg_graphql)
