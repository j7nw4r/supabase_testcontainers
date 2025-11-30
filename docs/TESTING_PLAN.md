# Integration Testing Implementation Plan

This document outlines the integration test scenarios for each Supabase service module. Use the checkboxes to track implementation progress.

---

## Overview

Each service requires Docker containers and follows a similar testing pattern:
1. Start required dependencies (usually PostgreSQL)
2. Configure and start the service container
3. Verify the service is healthy and functional
4. Test service-specific functionality
5. Cleanup (automatic via testcontainers)

### Common Testing Infrastructure

All integration tests share:
- `tokio::test` for async test execution
- `testcontainers::runners::AsyncRunner` for container management
- Unique network names per test for isolation
- `anyhow::Result` for error handling

---

## 1. PostgREST Integration Tests

**File:** `tests/postgrest_integration.rs`

**Dependencies:**
- PostgreSQL container (for database)
- PostgREST container

**Run command:** `cargo test --features postgrest,const --test postgrest_integration`

### Test Scenarios

- [x] **test_containers_start** - Verify PostgreSQL and PostgREST containers start successfully
  - Start PostgreSQL with test schema
  - Start PostgREST connected to PostgreSQL
  - Assert both containers have valid ports assigned

- [x] **test_health_endpoint_returns_200** - Verify PostgREST health/ready endpoint
  - Start containers
  - GET request to root endpoint `/`
  - Assert 200 status code

- [x] **test_openapi_schema_available** - Verify OpenAPI schema generation
  - Start containers
  - GET request to `/` with Accept: application/openapi+json
  - Assert response contains OpenAPI schema structure

- [x] **test_table_crud_operations** - Verify basic CRUD via REST API
  - Create a test table in PostgreSQL
  - POST to create a row
  - GET to retrieve the row
  - PATCH to update the row
  - DELETE to remove the row
  - Assert each operation succeeds

- [x] **test_anonymous_role_access** - Verify anon role restrictions
  - Create table with RLS policies
  - Make request without JWT
  - Assert request uses anon role correctly

- [x] **test_jwt_authentication** - Verify JWT-based authentication
  - Configure PostgREST with JWT secret
  - Create authenticated role in PostgreSQL
  - Make request with valid JWT
  - Assert role switching works correctly

- [x] **test_max_rows_limit** - Verify row limiting works
  - Configure PostgREST with max_rows=10
  - Insert 20 rows into test table
  - GET all rows
  - Assert only 10 rows returned

### Setup Requirements

```sql
-- Required PostgreSQL setup for PostgREST tests
CREATE ROLE anon NOLOGIN;
CREATE ROLE authenticator LOGIN PASSWORD 'testpass' NOINHERIT;
GRANT anon TO authenticator;

CREATE SCHEMA api;
CREATE TABLE api.todos (id SERIAL PRIMARY KEY, task TEXT, done BOOLEAN DEFAULT false);
GRANT USAGE ON SCHEMA api TO anon;
GRANT SELECT, INSERT, UPDATE, DELETE ON api.todos TO anon;
GRANT USAGE ON SEQUENCE api.todos_id_seq TO anon;
```

---

## 2. Storage Integration Tests

**File:** `tests/storage_integration.rs`

**Dependencies:**
- PostgreSQL container (for metadata storage)
- Storage container

**Run command:** `cargo test --features storage,const --test storage_integration`

**Note:** The storage-api container requires the complete Supabase PostgreSQL schema for full functionality. The tests below verify container setup patterns and basic connectivity. Full bucket/file operations require additional schema setup beyond what testcontainers can easily provide.

### Test Scenarios

- [x] **test_containers_start** - Verify containers start successfully
  - Start PostgreSQL with required roles
  - Start Storage connected to PostgreSQL
  - Assert both containers have valid ports

- [x] **test_health_endpoint_returns_200** - Verify health endpoint
  - Start containers
  - GET request to `/status`
  - Assert 200 status code

- [x] **test_file_size_limit_configuration** - Verify size limit configuration
  - Configure Storage with custom file size limit
  - Verify container starts with configuration
  - Assert health endpoint works

### Setup Requirements

Storage requires:
- PostgreSQL roles: `anon`, `authenticated`, `service_role`, `supabase_storage_admin`
- JWT tokens for authentication:
  - `ANON_KEY` - For public bucket access
  - `SERVICE_KEY` - For administrative operations

---

## 3. Analytics (Logflare) Integration Tests

**File:** `tests/analytics_integration.rs`

**Dependencies:**
- PostgreSQL container (for analytics storage)
- Analytics container

**Run command:** `cargo test --features analytics,const --test analytics_integration`

**Note:** The Logflare container's HTTP server only starts after migrations complete successfully. These tests verify container setup patterns and database connectivity.

### Test Scenarios

- [x] **test_containers_start** - Verify containers start successfully
  - Start PostgreSQL
  - Start Analytics with PostgreSQL backend
  - Assert containers have valid ports

- [x] **test_postgres_connection_works** - Verify PostgreSQL connectivity
  - Start containers
  - Connect directly to PostgreSQL
  - Verify query execution works

- [x] **test_analytics_schema_check** - Verify database schema
  - Connect to PostgreSQL directly
  - Check for `_analytics` schema presence
  - Log schema status

### Setup Requirements

Analytics requires:
- PostgreSQL backend URL
- Public access token (for log ingestion)
- Private access token (for management API)
- Base64 encryption key for sensitive data

---

## 4. Functions (Edge Runtime) Integration Tests

**File:** `tests/functions_integration.rs`

**Dependencies:**
- Functions container
- (Optional) PostgreSQL container for database access

**Run command:** `cargo test --features functions,const --test functions_integration`

**Note:** The edge-runtime container requires a functions directory to be mounted at startup. On SELinux-enabled systems (like Fedora), bind mounts require the `:Z` flag which testcontainers doesn't natively support. The tests below verify configuration patterns work correctly. Full function invocation tests require additional infrastructure setup.

### Test Scenarios

- [x] **test_default_configuration** - Verify default configuration is correct
  - Create Functions instance with defaults
  - Assert image name and tag are correct

- [x] **test_jwt_secret_configuration** - Verify JWT secret configuration
  - Configure Functions with JWT secret
  - Assert configuration is accepted

- [x] **test_full_configuration** - Verify all configuration options work
  - Configure Functions with all available options
  - Assert configuration is accepted

- [x] **test_exposed_port** - Verify correct port is exposed
  - Check exposed ports
  - Assert FUNCTIONS_PORT (9000) is exposed

- [x] **test_ready_conditions** - Verify ready conditions are correct
  - Check ready conditions
  - Assert waiting for "Listening on" message

- [x] **test_command** - Verify command is correctly set
  - Check container command
  - Assert "start --main-service /home/deno/functions"

- [x] **test_custom_main_service_path** - Verify custom path is respected
  - Configure with custom main service path
  - Assert path is included in command

### Setup Requirements

Functions require:
- Functions directory mounted into container at `/home/deno/functions`
- Each function in its own subdirectory with `index.ts`

Example test function structure:
```
tests/fixtures/functions/
├── hello/
│   └── index.ts
├── echo/
│   └── index.ts
└── db-query/
    └── index.ts
```

---

## 5. GraphQL (pg_graphql) Integration Tests

**File:** `tests/graphql_integration.rs`

**Dependencies:**
- PostgreSQL container (standard postgres image)
- PostgREST container (for HTTP access)

**Run command:** `cargo test --features graphql,postgrest,const --test graphql_integration`

**Note:** The `supabase/postgres` image (which includes pg_graphql) has known compatibility issues with testcontainers. The integration tests use standard PostgreSQL to verify the setup patterns work correctly. Full pg_graphql functionality requires the supabase/postgres image in a different testing environment (e.g., Docker Compose).

### Test Scenarios

- [x] **test_postgres_container_starts** - Verify PostgreSQL container starts
  - Start PostgreSQL container
  - Connect and execute simple query
  - Assert connection succeeds

- [x] **test_postgrest_connects_to_postgres** - Verify PostgREST connects to PostgreSQL
  - Start PostgreSQL and PostgREST containers
  - GET request to PostgREST root endpoint
  - Assert 200 status code

- [x] **test_rest_api_query** - Verify REST API query works
  - Create test table with data
  - GET request to table endpoint
  - Assert data is returned correctly

- [x] **test_rest_api_insert** - Verify REST API insert works
  - Create test table
  - POST request to insert data
  - Assert row is created

- [x] **test_direct_db_access** - Verify direct database access works
  - Start PostgreSQL and PostgREST
  - Connect directly to PostgreSQL
  - Assert can query alongside PostgREST

- [x] **test_openapi_schema** - Verify OpenAPI schema generation
  - Start PostgreSQL and PostgREST
  - GET request with OpenAPI accept header
  - Assert schema is returned

### Setup Requirements

The tests create roles and schema automatically:
```sql
-- Create roles for PostgREST
CREATE ROLE anon NOLOGIN;
CREATE ROLE authenticator LOGIN PASSWORD 'testpass' NOINHERIT;
GRANT anon TO authenticator;

-- Create API schema with test data
CREATE SCHEMA api;
CREATE TABLE api.todos (id SERIAL PRIMARY KEY, task TEXT, done BOOLEAN);
GRANT USAGE ON SCHEMA api TO anon;
GRANT SELECT, INSERT, UPDATE, DELETE ON api.todos TO anon;
```

---

## 6. Realtime Integration Tests

**File:** `tests/realtime_integration.rs`

**Dependencies:**
- PostgreSQL container (with logical replication enabled)
- Realtime container

**Run command:** `cargo test --features realtime,const --test realtime_integration`

**Note:** Full Realtime functionality (WebSocket connections, CDC) requires PostgreSQL with logical replication enabled (wal_level=logical). The Realtime container runs migrations on startup and requires additional configuration including `APP_NAME` and `RLIMIT_NOFILE`. The tests below verify configuration patterns work correctly. Full container startup tests require additional infrastructure setup.

### Test Scenarios

- [x] **test_default_configuration** - Verify default configuration is correct
  - Create Realtime instance with defaults
  - Assert image name and tag are correct

- [x] **test_postgres_connection_configuration** - Verify PostgreSQL connection config
  - Configure Realtime with connection string
  - Assert configuration is accepted

- [x] **test_individual_db_configuration** - Verify individual DB parameters
  - Configure with host, port, name, user, password, ssl
  - Assert configuration is accepted

- [x] **test_full_configuration** - Verify all configuration options
  - Configure with all available options
  - Assert configuration is accepted

- [x] **test_exposed_port** - Verify correct port is exposed
  - Check exposed ports
  - Assert REALTIME_PORT (4000) is exposed

- [x] **test_ready_conditions** - Verify ready conditions are correct
  - Check ready conditions
  - Assert waiting for "Realtime has started" message

- [x] **test_required_env_vars** - Verify required env vars are set
  - Check APP_NAME, RLIMIT_NOFILE, PORT, SLOT_NAME, etc.
  - Assert all required variables are set by default

- [x] **test_constructors** - Verify constructors work
  - Test new() and new_with_env() methods
  - Assert both work correctly

- [x] **test_slot_configuration** - Verify slot configuration options
  - Configure slot_name, temporary_slot, max_record_bytes
  - Assert configuration is accepted

- [x] **test_security_configuration** - Verify security configuration
  - Configure JWT secrets and secure_channels
  - Assert configuration is accepted

### Setup Requirements

PostgreSQL must have logical replication enabled:
```
wal_level = logical
max_replication_slots = 10
max_wal_senders = 10
```

For testing, use a PostgreSQL image that supports logical replication or configure via `postgresql.conf`.

Realtime requires:
- `DB_URL` or individual DB connection parameters
- `JWT_SECRET` for authentication
- `SECRET_KEY_BASE` for Phoenix (at least 64 chars)
- `APP_NAME` - required by Phoenix runtime config
- `RLIMIT_NOFILE` - required to avoid startup script error

---

## Implementation Notes

### Test Isolation

Each test should:
1. Generate unique network name using timestamp + counter
2. Use unique container names
3. Not depend on other tests' state

### Shared Test Utilities

Consider creating `tests/common/mod.rs` with:
- `unique_test_id()` function
- JWT generation helpers
- Common setup functions

### WebSocket Testing

For Realtime tests, add `tokio-tungstenite` as a dev dependency:
```toml
[dev-dependencies]
tokio-tungstenite = { version = "0.21", features = ["native-tls"] }
```

### Test Fixtures

Create `tests/fixtures/` directory for:
- SQL setup scripts
- Edge Functions code
- Sample files for Storage tests

---

## Dependencies to Add

```toml
[dev-dependencies]
testcontainers = { version = "0.25.2", features = ["default"] }
tokio = { version = "1.48.0", features = ["full", "test-util"] }
reqwest = { version = "0.12", features = ["json", "multipart"] }
serde_json = "1.0"
tokio-tungstenite = { version = "0.21", features = ["native-tls"] }
jsonwebtoken = "9.3"  # For JWT generation in tests
base64 = "0.22"       # For encoding test data
```

---

## Test Execution Order

Recommended order for implementation (simplest to most complex):

1. **GraphQL** - Just PostgreSQL, simple queries
2. **PostgREST** - PostgreSQL + REST API
3. **Storage** - PostgreSQL + file operations
4. **Analytics** - PostgreSQL + log ingestion
5. **Functions** - Standalone, needs function fixtures
6. **Realtime** - PostgreSQL + WebSocket (most complex)

---

## Success Criteria

All tests should:
- [ ] Pass independently (no shared state)
- [ ] Complete within reasonable timeout (< 60s each)
- [ ] Clean up resources automatically
- [ ] Provide clear failure messages
- [ ] Work in CI/CD environment with Docker
