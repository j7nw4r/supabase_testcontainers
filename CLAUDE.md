# CLAUDE.md

This file provides guidance to Claude Code (claude.ai/code) when working with code in this repository.

## Build and Test Commands

```bash
# Build the project
cargo build

# Build with all features
cargo build --all-features

# Build with specific feature
cargo build --features auth

# Run all tests
cargo test --all-features

# Run tests for a specific feature
cargo test --features auth

# Run a single test
cargo test --features auth test_auth_default

# Check code without building
cargo check --all-features

# Format code
cargo fmt

# Lint with clippy
cargo clippy --all-features
```

## Architecture

This crate provides testcontainers implementations for Supabase services. Each service module follows the same pattern:

1. **Module structure** (`src/<service>.rs`): Each service implements the `testcontainers::Image` trait with:
   - A struct holding `env_vars: HashMap<String, String>` for container configuration
   - `Default` impl with pre-configured environment variables
   - Builder methods for customization (e.g., `with_postgres_connection`)
   - `Image` trait impl defining `name()`, `tag()`, and `ready_conditions()`

2. **Feature-gated modules**: All service modules are conditionally compiled via Cargo features. The `lib.rs` re-exports public types based on enabled features.

3. **Shared constants** (`src/consts.rs`): Common values like `DOCKER_INTERNAL_HOST` and `LOCAL_HOST` for container networking.

## Key Patterns

- Services requiring PostgreSQL use `tokio_postgres` for database initialization (see `Auth::init_db_schema`)
- Container environment variables are stored in a `HashMap<String, String>` and exposed via the `Image::env_vars()` trait method
- Tests use `testcontainers::runners::AsyncRunner` with tokio for async container management

## Available Features

- `auth`, `postgrest`, `storage`, `analytics`, `functions`, `graphql`, `realtime` - Individual service containers
- `const` - Shared constants (auto-enabled by service features)
- `error` - Error types
- `postgres_testcontainer` - PostgreSQL container support
