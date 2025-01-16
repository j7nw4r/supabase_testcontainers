/*! Supabase PostgREST container management module.

This module provides functionality for managing Supabase PostgREST containers,
including initialization and configuration.
*/

use std::collections::HashMap;
use testcontainers_modules::testcontainers::core::WaitFor;
use testcontainers_modules::testcontainers::Image;

/// Default image name for PostgREST
const NAME: &str = "postgrest/postgrest";
/// Default image tag version
const TAG: &str = "v11.2.0";

/// Represents a PostgREST container configuration
#[derive(Debug, Clone)]
pub struct PostgREST {
    /// Environment variables to be passed to the container
    env_vars: HashMap<String, String>,
}

impl PostgREST {
    /// Creates a new PostgREST instance
    pub fn new() -> Self {
        Self::default()
    }

    /// Creates a new PostgREST instance with custom environment variables
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

    /// Sets the database schema
    pub fn with_db_schema(mut self, schema: &str) -> Self {
        self.env_vars.insert("PGRST_DB_SCHEMA".to_string(), schema.to_string());
        self
    }

    /// Sets the database anon role
    pub fn with_db_anon_role(mut self, role: &str) -> Self {
        self.env_vars.insert("PGRST_DB_ANON_ROLE".to_string(), role.to_string());
        self
    }
}

impl Default for PostgREST {
    fn default() -> Self {
        let mut env_vars = HashMap::new();
        // Add default environment variables here
        env_vars.insert("PGRST_DB_SCHEMA".to_string(), "public".to_string());
        env_vars.insert("PGRST_DB_ANON_ROLE".to_string(), "anon".to_string());
        env_vars.insert("PGRST_SERVER_PORT".to_string(), "3000".to_string());
        Self { env_vars }
    }
}

impl Image for PostgREST {
    fn name(&self) -> &str {
        NAME
    }

    fn tag(&self) -> &str {
        TAG
    }

    fn ready_conditions(&self) -> Vec<WaitFor> {
        vec![WaitFor::message_on_stdout("PostgREST server listening")]
    }
}

#[cfg(test)]
#[cfg(feature = "postgrest")]
mod test {
    use super::*;

    #[test]
    fn test_postgrest_default() {
        let postgrest = PostgREST::default();
        assert_eq!(postgrest.name(), NAME);
        assert_eq!(postgrest.tag(), TAG);
        assert_eq!(postgrest.env_vars.get("PGRST_DB_SCHEMA"), Some(&"public".to_string()));
        assert_eq!(postgrest.env_vars.get("PGRST_DB_ANON_ROLE"), Some(&"anon".to_string()));
    }
}