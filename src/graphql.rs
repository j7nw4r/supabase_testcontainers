/*! Supabase GraphQL container management module.

This module provides functionality for managing Supabase GraphQL (pg_graphql) containers,
including initialization and configuration.
*/

use std::collections::HashMap;
use testcontainers_modules::testcontainers::core::WaitFor;
use testcontainers_modules::testcontainers::Image;

/// Default image name for Supabase GraphQL
const NAME: &str = "supabase/pg_graphql";
/// Default image tag version
const TAG: &str = "latest";

/// Represents a Supabase GraphQL container configuration
#[derive(Debug, Clone)]
pub struct GraphQL {
    /// Environment variables to be passed to the container
    env_vars: HashMap<String, String>,
}

impl GraphQL {
    /// Creates a new GraphQL instance
    pub fn new() -> Self {
        Self::default()
    }

    /// Creates a new GraphQL instance with custom environment variables
    pub fn new_with_env(envs: HashMap<&str, &str>) -> Self {
        let mut instance = Self::default();
        for (key, val) in envs {
            instance.env_vars.insert(key.to_string(), val.to_string());
        }
        instance
    }

    /// Sets the PostgreSQL connection string
    pub fn with_postgres_connection(mut self, connection_string: &str) -> Self {
        self.env_vars.insert("DATABASE_URL".to_string(), connection_string.to_string());
        self
    }
}

impl Default for GraphQL {
    fn default() -> Self {
        let mut env_vars = HashMap::new();
        // Add default environment variables here
        env_vars.insert("GRAPHQL_PORT".to_string(), "5000".to_string());
        Self { env_vars }
    }
}

impl Image for GraphQL {
    fn name(&self) -> &str {
        NAME
    }

    fn tag(&self) -> &str {
        TAG
    }

    fn ready_conditions(&self) -> Vec<WaitFor> {
        vec![WaitFor::message_on_stdout("GraphQL server started")]
    }
}

#[cfg(test)]
#[cfg(feature = "graphql")]
mod test {
    use super::*;

    #[test]
    fn test_graphql_default() {
        let graphql = GraphQL::default();
        assert_eq!(graphql.name(), NAME);
        assert_eq!(graphql.tag(), TAG);
        assert_eq!(graphql.env_vars.get("GRAPHQL_PORT"), Some(&"5000".to_string()));
    }
}