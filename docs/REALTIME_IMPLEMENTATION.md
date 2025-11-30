# Realtime Feature Implementation Plan

This document tracks the implementation of the `realtime` feature for the Supabase testcontainers crate.

## Overview

Supabase Realtime is a server that enables real-time functionality through:
- PostgreSQL logical replication (CDC - Change Data Capture)
- WebSocket connections for broadcasting changes
- Presence tracking for connected users
- Broadcast messaging between clients

**Docker Image:** `supabase/realtime`
**Default Port:** 4000

## Implementation Sections

### Section 1: Core Struct Definition
- [x] Change `env_vars` from `HashMap` to `BTreeMap`
- [x] Add `tag: String` field for version customization
- [x] Define `REALTIME_PORT` constant (4000)
- [x] Update module constants (NAME, TAG)

### Section 2: Default Implementation
- [x] Configure server port (PORT=4000)
- [x] Configure replication slot settings (SLOT_NAME, TEMPORARY_SLOT)
- [x] Add security defaults (SECURE_CHANNELS, JWT_CLAIM_VALIDATORS)
- [x] Configure region and tenant settings
- [x] Set appropriate log level for testing

### Section 3: Builder Methods
- [x] `with_postgres_connection` - Database URL
- [x] `with_jwt_secret` - JWT signing secret
- [x] `with_tag` - Custom Docker image version
- [x] `with_env` - Generic environment variable
- [x] `with_slot_name` - Replication slot name
- [x] `with_temporary_slot` - Temporary slot setting
- [x] `with_region` - AWS region setting
- [x] `with_tenant_id` - Tenant identifier
- [x] `with_secure_channels` - Secure channels setting
- [x] `with_api_jwt_secret` - API JWT secret
- [x] `with_secret_key_base` - Phoenix secret key base
- [x] `with_db_host` / `with_db_port` / `with_db_name` / `with_db_user` / `with_db_password` - Individual DB config

### Section 4: Image Trait Implementation
- [x] Implement `name()` returning "supabase/realtime"
- [x] Implement `tag()` returning custom tag
- [x] Implement `ready_conditions()` with appropriate log message
- [x] Implement `expose_ports()` returning REALTIME_PORT
- [x] Implement `env_vars()` returning BTreeMap iterator
- [x] Implement `exec_after_start()` (empty, for future use)

### Section 5: Library Exports
- [x] Update `lib.rs` to export `REALTIME_PORT` with `Realtime`

### Section 6: Module Documentation
- [x] Add comprehensive module-level rustdoc
- [x] Include usage examples
- [x] Document all configuration options
- [x] Add struct-level documentation

### Section 7: Unit Tests
- [x] Test default configuration values
- [x] Test `name()` returns correct image
- [x] Test `tag()` returns correct version
- [x] Test `REALTIME_PORT` constant
- [x] Test each builder method
- [x] Test builder method chaining
- [x] Test `expose_ports()` returns correct port
- [x] Test `ready_conditions()` is non-empty

### Section 8: Validation
- [x] Run `cargo fmt`
- [x] Run `cargo clippy --all-features`
- [x] Run `cargo test --features realtime`
- [x] Run `cargo build --features realtime`

## Environment Variables Reference

Based on Supabase Realtime configuration:

| Variable | Description | Default |
|----------|-------------|---------|
| `PORT` | Server port | 4000 |
| `DB_HOST` | PostgreSQL host | - |
| `DB_PORT` | PostgreSQL port | 5432 |
| `DB_NAME` | Database name | postgres |
| `DB_USER` | Database user | supabase_admin |
| `DB_PASSWORD` | Database password | - |
| `DB_SSL` | Enable SSL | false |
| `DB_AFTER_CONNECT_QUERY` | Query after connect | - |
| `SLOT_NAME` | Replication slot name | realtime_rls |
| `TEMPORARY_SLOT` | Use temporary slot | true |
| `JWT_SECRET` | JWT signing secret | - |
| `API_JWT_SECRET` | API JWT secret | - |
| `SECRET_KEY_BASE` | Phoenix secret key | - |
| `SECURE_CHANNELS` | Enable secure channels | true |
| `REGION` | AWS region | local |
| `TENANT_ID` | Tenant identifier | realtime-dev |
| `ERL_AFLAGS` | Erlang flags | -proto_dist inet_tcp |
| `DNS_NODES` | DNS cluster nodes | - |
| `ENABLE_TAILSCALE` | Enable Tailscale | false |

## Commit History

| Section | Commit | Status |
|---------|--------|--------|
| Section 1-4, 6-7 | 14be1dc | Complete |
| Section 5 | 381eb32 | Complete |
| Section 8 | - | Complete (validated) |

## Notes

- The Realtime server uses Phoenix/Elixir and requires specific Erlang flags
- Replication slots are used for CDC (Change Data Capture)
- `TEMPORARY_SLOT=true` is recommended for testing to avoid slot accumulation
- The ready condition should match the actual startup log message from the container
