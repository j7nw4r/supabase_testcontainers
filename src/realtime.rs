/*! Supabase Realtime container management module.

This module provides functionality for managing Supabase Realtime containers,
including initialization and configuration.
*/

use std::collections::HashMap;
use testcontainers_modules::testcontainers::core::WaitFor;
use testcontainers_modules::testcontainers::Image;

/// Default image name for Supabase Realtime
const NAME: &str = "supabase/realtime";
/// Default image tag version
const TAG: &str = "latest";

/// Represents a Supabase Realtime container configuration
#[derive(Debug, Clone)]
pub struct Realtime {
    /// Environment variables to be passed to the container
    env_vars: HashMap<String, String>,
}

impl Realtime {
    /// Creates a new Realtime instance
    pub fn new() -> Self {
        Self::default()
    }

    /// Creates a new Realtime instance with custom environment variables
    pub fn new_with_env(envs: HashMap<&str, &str>) -> Self {
        let mut instance = Self::default();
        for (key, val) in envs {
            instance.env_vars.insert(key.to_string(), val.to_string());
        }
        instance
    }

    /// Sets the PostgreSQL connection string
    pub fn with_postgres_connection(mut self, connection_string: &str) -> Self {
        self.env_vars.insert("DB_URL".to_string(), connection_string.to_string());
        self
    }

    /// Sets the JWT secret
    pub fn with_jwt_secret(mut self, secret: &str) -> Self {
        self.env_vars.insert("JWT_SECRET".to_string(), secret.to_string());
        self
    }
}

impl Default for Realtime {
    fn default() -> Self {
        let mut env_vars = HashMap::new();
        // Add default environment variables here
        env_vars.insert("PORT".to_string(), "4000".to_string());
        env_vars.insert("SLOT_NAME".to_string(), "realtime_rls".to_string());
        env_vars.insert("TEMPORARY_SLOT".to_string(), "true".to_string());
        Self { env_vars }
    }
}

impl Image for Realtime {
    fn name(&self) -> &str {
        NAME
    }

    fn tag(&self) -> &str {
        TAG
    }

    fn ready_conditions(&self) -> Vec<WaitFor> {
        vec![WaitFor::message_on_stdout("Realtime server started")]
    }
}

#[cfg(test)]
#[cfg(feature = "realtime")]
mod test {
    use super::*;

    #[test]
    fn test_realtime_default() {
        let realtime = Realtime::default();
        assert_eq!(realtime.name(), NAME);
        assert_eq!(realtime.tag(), TAG);
        assert_eq!(realtime.env_vars.get("PORT"), Some(&"4000".to_string()));
        assert_eq!(realtime.env_vars.get("SLOT_NAME"), Some(&"realtime_rls".to_string()));
    }
}