/*! Supabase Analytics container management module.

This module provides functionality for managing Supabase Analytics containers,
including initialization and configuration.
*/

use std::collections::HashMap;
use testcontainers_modules::testcontainers::core::WaitFor;
use testcontainers_modules::testcontainers::Image;

/// Default image name for Supabase Analytics
const NAME: &str = "supabase/analytics";
/// Default image tag version
const TAG: &str = "latest";

/// Represents a Supabase Analytics container configuration
#[derive(Debug, Clone)]
pub struct Analytics {
    /// Environment variables to be passed to the container
    env_vars: HashMap<String, String>,
}

impl Analytics {
    /// Creates a new Analytics instance
    pub fn new() -> Self {
        Self::default()
    }

    /// Creates a new Analytics instance with custom environment variables
    pub fn new_with_env(envs: HashMap<&str, &str>) -> Self {
        let mut instance = Self::default();
        for (key, val) in envs {
            instance.env_vars.insert(key.to_string(), val.to_string());
        }
        instance
    }
}

impl Default for Analytics {
    fn default() -> Self {
        let env_vars = HashMap::new();
        // Add default environment variables here
        Self { env_vars }
    }
}

impl Image for Analytics {
    fn name(&self) -> &str {
        NAME
    }

    fn tag(&self) -> &str {
        TAG
    }

    fn ready_conditions(&self) -> Vec<WaitFor> {
        vec![WaitFor::message_on_stdout("Analytics service started")]
    }
}

#[cfg(test)]
#[cfg(feature = "analytics")]
mod test {
    use super::*;

    #[test]
    fn test_analytics_default() {
        let analytics = Analytics::default();
        assert_eq!(analytics.name(), NAME);
        assert_eq!(analytics.tag(), TAG);
    }
}