# Functions Feature Implementation Plan

This document tracks the implementation of the `functions` feature for the Supabase testcontainers crate.

## Overview

Supabase Edge Functions is a Deno-based runtime that enables serverless JavaScript/TypeScript/WASM functions:
- Deno runtime for secure, isolated function execution
- JWT authentication for function invocation
- Direct database access via `SUPABASE_DB_URL`
- Integration with Supabase services via API keys

**Docker Image:** `supabase/edge-runtime`
**Default Port:** 9000

## Implementation Sections

### Section 1: Core Struct Definition
- [x] Change `env_vars` from `HashMap` to `BTreeMap`
- [x] Add `tag: String` field for version customization
- [x] Define `FUNCTIONS_PORT` constant (9000)
- [x] Update module constants (NAME, TAG with specific version)

### Section 2: Default Implementation
- [x] Configure server port (PORT=9000)
- [x] Set VERIFY_JWT default (true for security)
- [x] Configure main service path
- [x] Set appropriate defaults for testing

### Section 3: Builder Methods
- [x] `with_jwt_secret` - JWT signing secret
- [x] `with_supabase_url` - Supabase API URL
- [x] `with_anon_key` - Anonymous JWT key
- [x] `with_service_role_key` - Service role JWT key
- [x] `with_db_url` - PostgreSQL connection string
- [x] `with_verify_jwt` - Enable/disable JWT verification
- [x] `with_main_service_path` - Functions directory path
- [x] `with_port` - Server port
- [x] `with_tag` - Custom Docker image version
- [x] `with_env` - Generic environment variable
- [x] `with_worker_timeout_ms` - Worker timeout configuration
- [x] `with_max_parallelism` - Maximum concurrent workers

### Section 4: Image Trait Implementation
- [x] Implement `name()` returning "supabase/edge-runtime"
- [x] Implement `tag()` returning custom tag
- [x] Implement `ready_conditions()` with appropriate log message
- [x] Implement `expose_ports()` returning FUNCTIONS_PORT
- [x] Implement `env_vars()` returning BTreeMap iterator
- [x] Implement `exec_after_start()` (empty, for future use)
- [x] Implement `cmd()` for startup command with --main-service flag

### Section 5: Library Exports
- [x] Update `lib.rs` to export `FUNCTIONS_PORT` with `Functions`

### Section 6: Module Documentation
- [x] Add comprehensive module-level rustdoc
- [x] Include usage examples
- [x] Document all configuration options
- [x] Add struct-level documentation

### Section 7: Unit Tests
- [x] Test default configuration values
- [x] Test `name()` returns correct image
- [x] Test `tag()` returns correct version
- [x] Test `FUNCTIONS_PORT` constant
- [x] Test each builder method
- [x] Test builder method chaining
- [x] Test `expose_ports()` returns correct port
- [x] Test `ready_conditions()` is non-empty
- [x] Test `cmd()` returns correct startup command

### Section 8: Validation
- [x] Run `cargo fmt`
- [x] Run `cargo clippy --all-features`
- [x] Run `cargo test --features functions`
- [x] Run `cargo build --features functions`

## Environment Variables Reference

Based on Supabase Edge Runtime configuration:

| Variable | Description | Default |
|----------|-------------|---------|
| `PORT` | Server port | 9000 |
| `JWT_SECRET` | JWT signing secret | - |
| `SUPABASE_URL` | Supabase API URL (Kong gateway) | - |
| `SUPABASE_ANON_KEY` | Anonymous JWT key | - |
| `SUPABASE_SERVICE_ROLE_KEY` | Service role JWT key | - |
| `SUPABASE_DB_URL` | PostgreSQL connection string | - |
| `VERIFY_JWT` | Verify JWT on invocation | true |

## Startup Command

The edge-runtime requires a startup command:
```
start --main-service /home/deno/functions
```

This tells the runtime where to find the functions to serve.

## Commit History

| Section | Commit | Status |
|---------|--------|--------|
| Section 1-4, 6-7 | - | Complete |
| Section 5 | - | Complete |
| Section 8 | - | Complete (validated) |

## Notes

- Edge Functions run in a Deno-based sandbox environment
- The runtime supports JavaScript, TypeScript, and WASM
- Functions are loaded from the `/home/deno/functions` directory by default
- JWT verification should be enabled for production but can be disabled for testing
- The `SUPABASE_SERVICE_ROLE_KEY` bypasses Row Level Security

## Sources

- [Supabase Self-Hosting Docker Docs](https://supabase.com/docs/guides/self-hosting/docker)
- [Supabase Edge Runtime GitHub](https://github.com/supabase/edge-runtime)
- [Self-Hosting Functions Reference](https://supabase.com/docs/reference/self-hosting-functions/introduction)
