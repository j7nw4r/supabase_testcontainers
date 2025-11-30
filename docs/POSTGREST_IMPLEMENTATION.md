# PostgREST Implementation Tracking

This document tracks the implementation progress for enhancing the PostgREST module.

## Sections

- [x] **Section 1: Core Structure Updates**
  - Switch from `HashMap` to `BTreeMap` for deterministic ordering
  - Add `tag` field for version override
  - Add public `POSTGREST_PORT` constant
  - Update imports

- [x] **Section 2: Builder Methods**
  - `with_jwt_secret()` - JWT validation secret
  - `with_jwt_role_claim_key()` - Custom role claim path
  - `with_openapi_mode()` - OpenAPI spec generation mode
  - `with_max_rows()` - Maximum rows returned
  - `with_pre_request()` - Pre-request function
  - `with_log_level()` - Logging verbosity
  - `with_tag()` - Image version override
  - `with_env()` - Custom environment variable escape hatch

- [x] **Section 3: Complete Image Trait Implementation**
  - Implement `expose_ports()`
  - Implement `env_vars()` with proper return type
  - Implement `exec_after_start()`

- [x] **Section 4: Documentation**
  - Module-level documentation with example
  - Struct documentation
  - Document all builder methods

- [x] **Section 5: Comprehensive Tests**
  - Test default values
  - Test each builder method
  - Test port constant
  - Test Image trait methods

---

## Progress Log

| Section | Status | Commit |
|---------|--------|--------|
| Section 1 | Complete | 30768e7 |
| Section 2 | Complete | 3b86420 |
| Section 3 | Complete | 5d775f3 |
| Section 4 | Complete | 44b77f3 |
| Section 5 | Complete | 65e344f |
