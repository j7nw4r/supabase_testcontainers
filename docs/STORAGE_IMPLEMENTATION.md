# Storage Implementation Tracking

This document tracks the implementation progress for enhancing the Storage module.

## Sections

- [x] **Section 1: Core Structure Updates**
  - Switch from `HashMap` to `BTreeMap` for deterministic ordering
  - Add `tag` field for version override
  - Add public `STORAGE_PORT` constant (5000)
  - Update to specific image version (not `latest`)
  - Fix incorrect environment variable names
  - Update imports for full Image trait

- [x] **Section 2: Builder Methods**
  - `with_database_url()` - PostgreSQL connection string
  - `with_anon_key()` - Anonymous JWT token
  - `with_service_key()` - Service role JWT token
  - `with_jwt_secret()` - JWT secret for validation
  - `with_postgrest_url()` - PostgREST server URL
  - `with_tenant_id()` - Storage tenant identifier
  - `with_region()` - S3/storage region
  - `with_global_s3_bucket()` - S3 bucket name
  - `with_file_size_limit()` - Maximum file size
  - `with_storage_backend()` - Backend type (file, s3)
  - `with_file_storage_path()` - Local file storage path
  - `with_upload_signed_url_expiration()` - Signed URL expiry
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
| Section 1 | Complete | e0349ce |
| Section 2 | Complete | d56267c |
| Section 3 | Complete | c74234c |
| Section 4 | Complete | 193ef10 |
| Section 5 | Complete | 7fdae97 |
