# Repository Guidelines

## Project Structure & Module Organization
- Crate source lives in `src/` with one module per Supabase service (`auth.rs`, `postgrest.rs`, `storage.rs`, `functions.rs`, `graphql.rs`, `realtime.rs`, `analytics.rs`); `lib.rs` re-exports them.
- Shared constants and errors sit in `consts.rs` and `error.rs`; add cross-cutting values there before duplicating literals.
- Integration coverage is in `tests/auth_integration.rs` and spins up real containers via Testcontainers, so Docker must be running.
- Reference docs live in `docs/`; `README.md` shows usage patterns. Build artifacts land in `target/` and stay untracked.

## Build, Test, and Development Commands
- `cargo build --all-features` — compile with every service flag to catch cfg gaps; narrow scope with `--features "<list>"` when iterating.
- `cargo fmt --all` — format with rustfmt; run before pushing.
- `cargo clippy --all-targets --all-features -D warnings` — lint everything; prefer fixes over `allow` unless unavoidable.
- `cargo test --features auth,const --test auth_integration -- --nocapture` — integration tests against real containers; first run pulls images and may take minutes.

## Coding Style & Naming Conventions
- Rustfmt defaults (4-space indent); keep imports explicit and prefer early returns over deep nesting.
- Modules/functions use `snake_case`; public structs/enums use `PascalCase`; constants use `SCREAMING_SNAKE_CASE`.
- Keep feature gates tight and named after services (`auth`, `postgrest`, `storage`, etc.); document new flags in `Cargo.toml` and `README.md`.
- Use `anyhow::Result` for setup paths and `thiserror` for user-facing error types; add short doc comments for public items.

## Testing Guidelines
- Docker is mandatory; tests start PostgreSQL and service containers.
- Name new integration files `tests/<service>_integration.rs` and test functions `test_<behavior>` for clarity.
- Include a health or behavior check per service (e.g., `/health` plus one request).
- For parallel runs, generate unique networks/container names (see `unique_test_id()` in the Auth suite) to avoid port collisions.

## Commit & Pull Request Guidelines
- Follow Conventional Commits used here (`feat`, `docs`, `chore`, `fix`, optional scopes); keep messages imperative.
- PRs should note behavior changes, enabled features (if not `--all-features`), and commands run (fmt, clippy, tests).
- Link related issues and attach logs or screenshots for user-visible changes; keep diffs focused and split unrelated tweaks.

## Security & Configuration Tips
- Never commit real credentials; mirror the test pattern of generated passwords and isolated networks.
- Pin explicit image tags instead of `latest`; match versions documented in `README.md`.
- Prefer Testcontainers-managed dynamic ports; expose only what the test actually needs.
