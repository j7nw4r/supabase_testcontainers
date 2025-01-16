/*! Supabase Storage container management module.

This module provides functionality for managing Supabase Storage containers,
including initialization and configuration.
*/

use std::collections::HashMap;
use testcontainers_modules::testcontainers::core::WaitFor;
use testcontainers_modules::testcontainers::Image;

/// Default image name for Supabase Storage
const NAME: &str = "supabase/storage-api";
/// Default image tag version
const TAG: &str = "latest";

/// Represents a Supabase Storage container configuration
#[derive(Debug, Clone)]
pub struct Storage {
    /// Environment variables to be passed to the container
    env_vars: HashMap<String, String>,
}

impl Storage {
    /// Creates a new Storage instance
    pub fn new() -> Self {
        Self::default()
    }

    /// Creates a new Storage instance with custom environment variables
    pub fn new_with_env(envs: HashMap<&str, &str>) -> Self {
        let mut instance = Self::default();
        for (key, val) in envs {
            instance.env_vars.insert(key.to_string(), val.to_string());
        }
        instance
    }

    /// Sets the PostgreSQL connection string
    pub fn with_postgres_connection(mut self, connection_string: &str) -> Self {
        self.env_vars.insert("PGRST_DB_URI".to_string(), connection_string.to_string());
        self
    }

    /// Sets the JWT secret
    pub fn with_jwt_secret(mut self, secret: &str) -> Self {
        self.env_vars.insert("ANON_KEY".to_string(), secret.to_string());
        self.env_vars.insert("SERVICE_ROLE_KEY".to_string(), secret.to_string());
        self
    }

    /// Sets the storage backend (local, s3, etc.)
    pub fn with_storage_backend(mut self, backend: &str) -> Self {
        self.env_vars.insert("STORAGE_BACKEND".to_string(), backend.to_string());
        self
    }
}

impl Default for Storage {
    fn default() -> Self {
        let mut env_vars = HashMap::new();
        // Add default environment variables here
        env_vars.insert("REGION".to_string(), "local".to_string());
        env_vars.insert("STORAGE_BACKEND".to_string(), "file".to_string());
        env_vars.insert("FILE_SIZE_LIMIT".to_string(), "52428800".to_string()); // 50MB
        env_vars.insert("GLOBAL_S3_BUCKET".to_string(), "storage".to_string());
        env_vars.insert("PORT".to_string(), "5000".to_string());
        Self { env_vars }
    }
}

impl Image for Storage {
    fn name(&self) -> &str {
        NAME
    }

    fn tag(&self) -> &str {
        TAG
    }

    fn ready_conditions(&self) -> Vec<WaitFor> {
        vec![WaitFor::message_on_stdout("Storage API server started")]
    }
}

#[cfg(test)]
#[cfg(feature = "storage")]
mod test {
    use super::*;

    #[test]
    fn test_storage_default() {
        let storage = Storage::default();
        assert_eq!(storage.name(), NAME);
        assert_eq!(storage.tag(), TAG);
        assert_eq!(storage.env_vars.get("STORAGE_BACKEND"), Some(&"file".to_string()));
        assert_eq!(storage.env_vars.get("REGION"), Some(&"local".to_string()));
    }
}