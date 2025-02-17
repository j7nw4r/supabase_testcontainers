# Supabase-Testcontainers

> ⚠️ **Development Status**: This crate is currently under active development and is not yet ready for production use. APIs may change significantly between versions.

A Rust implementation of [Testcontainers](https://github.com/testcontainers/testcontainers-rs) for [Supabase](https://supabase.com/) services. This crate provides utilities for setting up and managing Supabase services in a containerized environment, primarily for testing purposes.

## Installation

Add this to your `Cargo.toml`:

```toml
[dependencies]
supabase-testcontainers-modules = "0.1.0"
```

Or with specific features:

```toml
[dependencies]
supabase-testcontainers-modules = { version = "0.1.0", features = ["auth"] }
```

## Features

- `analytics` - Enables the Supabase Analytics service container support
- `auth` - Enables the Supabase Auth service container support
- `const` - Provides constant values used throughout the crate
- `error` - Enables error handling functionality
- `functions` - Enables the Edge Functions service container support
- `graphql` - Enables the GraphQL (pg_graphql) service container support
- `postgrest` - Enables the PostgREST service container support
- `postgres_testcontainer` - Enables PostgreSQL container support
- `realtime` - Enables the Realtime service container support
- `storage` - Enables the Storage service container support

## Usage

### Basic Setup

```rust
use supabase_testcontainers_modules::Auth;
use testcontainers::Docker;

#[tokio::main]
async fn main() {
    // Create a new Docker instance
    let docker = Docker::new();
    
    // Start an Auth container
    let auth = Auth::new(&docker);
    
    // The container is now running and ready for testing
    // You can access it through the provided methods
}
```

### With PostgreSQL

When using the `postgres_testcontainer` feature:

```rust
use supabase_testcontainers_modules::Auth;
use testcontainers::Docker;

#[tokio::main]
async fn main() {
    let docker = Docker::new();
    let auth = Auth::new(&docker)
        .with_postgres() // This will automatically set up and configure PostgreSQL
        .await;
        
    // Both Auth and PostgreSQL containers are now running
}
```

## Current Status

The following Supabase services are implemented or planned:

- [ ] Analytics
- [x] Auth (partially implemented)
- [ ] Edge Functions
- [ ] GraphQL (pg_graphql)
- [ ] PostgREST
- [ ] Realtime
- [ ] Storage

## Contributing

Contributions are welcome! Please feel free to submit a Pull Request.

## License

This project is licensed under the MIT License.
