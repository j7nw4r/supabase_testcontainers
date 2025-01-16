/*! Supabase Functions container management module.

This module provides functionality for managing Supabase Edge Functions containers,
including initialization and configuration.
*/

use std::collections::HashMap;
use testcontainers_modules::testcontainers::core::WaitFor;
use testcontainers_modules::testcontainers::Image;

/// Default image name for Supabase Functions
const NAME: &str = "supabase/edge-runtime";
/// Default image tag version
const TAG: &str = "latest";

/// Represents a Supabase Functions container configuration
#[derive(Debug, Clone)]
pub struct Functions {
    /// Environment variables to be passed to the container
    env_vars: HashMap<String, String>,
}

impl Functions {
    /// Creates a new Functions instance
    pub fn new() -> Self {
        Self::default()
    }

    /// Creates a new Functions instance with custom environment variables
    pub fn new_with_env(envs: HashMap<&str, &str>) -> Self {
        let mut instance = Self::default();
        for (key, val) in envs {
            instance.env_vars.insert(key.to_string(), val.to_string());
        }
        instance
    }
}

impl Default for Functions {
    fn default() -> Self {
        let mut env_vars = HashMap::new();
        // Add default environment variables here
        env_vars.insert("FUNCTIONS_PORT".to_string(), "9000".to_string());
        Self { env_vars }
    }
}

impl Image for Functions {
    fn name(&self) -> &str {
        NAME
    }

    fn tag(&self) -> &str {
        TAG
    }

    fn ready_conditions(&self) -> Vec<WaitFor> {
        vec![WaitFor::message_on_stdout("Edge Functions runtime ready")]
    }
}

#[cfg(test)]
#[cfg(feature = "functions")]
mod test {
    use super::*;

    #[test]
    fn test_functions_default() {
        let functions = Functions::default();
        assert_eq!(functions.name(), NAME);
        assert_eq!(functions.tag(), TAG);
        assert_eq!(functions.env_vars.get("FUNCTIONS_PORT"), Some(&"9000".to_string()));
    }
}