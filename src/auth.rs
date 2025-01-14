use std::borrow::Cow;
use std::collections::HashMap;

use anyhow::{bail, Context};
use testcontainers_modules::testcontainers::core::{ContainerState, ExecCommand, WaitFor};
use testcontainers_modules::testcontainers::{Image, TestcontainersError};
use tokio_postgres::NoTls;

const NAME: &str = "supabase/auth";
const TAG: &str = "v2.164.0";

#[derive(Debug, Clone)]
pub struct Auth {
    env_vars: HashMap<String, String>,
}

impl Auth {
    pub fn new(postgres_connection_string: &str) -> Self {
        let mut default_image = Self::default();
        default_image.env_vars.insert(
            "DATABASE_URL".to_string(),
            postgres_connection_string.to_owned(),
        );
        default_image
    }

    pub fn new_with_env(envs: HashMap<&str, &str>) -> Self {
        let mut default_image = Self::default();
        for (key, val) in envs {
            default_image
                .env_vars
                .insert(key.to_string(), val.to_string());
        }
        default_image
    }

    pub fn git_release_version(&self) -> String {
        let version = TAG[1..].to_string();
        format!("release/{}", version)
    }
    pub async fn init_db_schema(self, db_url: &str) -> anyhow::Result<Self> {
        if db_url.is_empty() {
            bail!("db_url cannot be empty");
        }
        let Some(db_schema) = self.env_vars.get("DB_NAMESPACE") else {
            bail!("DB_NAMESPACE env var is not set");
        };
        if db_schema.is_empty() {
            bail!("DB_NAMESPACE cannot be empty");
        }

        println!("retrieving migrations for {}", self.git_release_version());

        let (client, connection) = tokio_postgres::connect(db_url, NoTls)
            .await
            .context("could not connect to postgres db")?;

        /*
        The connection object performs the actual communication with the database, so spawn it off
        to run on its own.
         */
        tokio::spawn(async move {
            if let Err(e) = connection.await {
                eprintln!("connection error: {}", e);
            }
        });

        let query = format!(
            "CREATE USER supabase_admin LOGIN CREATEROLE CREATEDB REPLICATION BYPASSRLS;
        -- Supabase super admin
        CREATE USER supabase_auth_admin NOINHERIT CREATEROLE LOGIN NOREPLICATION PASSWORD 'root';
        CREATE SCHEMA IF NOT EXISTS {db_schema} AUTHORIZATION supabase_auth_admin;
        GRANT CREATE ON DATABASE postgres TO supabase_auth_admin;
        ALTER USER supabase_auth_admin SET search_path = '{db_schema}';"
        );
        client.batch_execute(&query).await?;

        Ok(self)
    }
}
impl Default for Auth {
    fn default() -> Self {
        let mut env_vars = HashMap::new();
        env_vars.insert(
            "DATABASE_URL".to_string(),
            "postgres://postgres:postgres@host.docker.internal:5432/postgres?sslmode=disable"
                .to_string(),
        );
        env_vars.insert(
            "GOTRUE_JWT_SECRET".to_string(),
            "37c304f8-51aa-419a-a1af-06154e63707a".to_string(),
        );
        env_vars.insert("GOTRUE_JWT_EXP".to_string(), "3600".to_string());
        env_vars.insert("GOTRUE_DB_DRIVER".to_string(), "postgres".to_string());
        env_vars.insert("DB_NAMESPACE".to_string(), "auth".to_string());
        env_vars.insert(
            "API_EXTERNAL_URL".to_string(),
            "http://localhost".to_string(),
        );
        env_vars.insert("GOTRUE_API_HOST".to_string(), "0.0.0.0".to_string());
        env_vars.insert("PORT".to_string(), "9999".to_string());
        env_vars.insert("GOTRUE_DISABLE_SIGNUP".to_string(), "false".to_string());
        env_vars.insert(
            "GOTRUE_SITE_URL".to_string(),
            "http://localhost".to_string(),
        );
        env_vars.insert("GOTRUE_LOG_LEVEL".to_string(), "DEBUG".to_string());
        env_vars.insert(
            "GOTRUE_OPERATOR_TOKEN".to_string(),
            "super-secret-operator-token".to_string(),
        );
        env_vars.insert(
            "GOTRUE_EXTERNAL_PHONE_ENABLED".to_string(),
            "true".to_string(),
        );
        env_vars.insert("GOTRUE_MAILER_AUTOCONFIRM".to_string(), "true".to_string());
        env_vars.insert("GOTRUE_SMS_AUTOCONFIRM".to_string(), "true".to_string());
        env_vars.insert("GOTRUE_SMS_PROVIDER".to_string(), "twilio".to_string());
        env_vars.insert(
            "GOTRUE_EXTERNAL_ANONYMOUS_USERS_ENABLED".to_string(),
            "true".to_string(),
        );
        env_vars.insert(
            "GOTRUE_EXTERNAL_GITHUB_ENABLED".to_string(),
            "true".to_string(),
        );
        env_vars.insert(
            "GOTRUE_EXTERNAL_GITHUB_CLIENT_ID".to_string(),
            "myappclientid".to_string(),
        );
        env_vars.insert(
            "GOTRUE_EXTERNAL_GITHUB_SECRET".to_string(),
            "clientsecretvaluessssh".to_string(),
        );
        env_vars.insert(
            "GOTRUE_EXTERNAL_GITHUB_REDIRECT_URI".to_string(),
            "http://localhost:3000/callback".to_string(),
        );
        env_vars.insert(
            "GOTRUE_SECURITY_MANUAL_LINKING_ENABLED".to_string(),
            "true".to_string(),
        );
        env_vars.insert(
            "PATH".to_string(),
            "/usr/local/sbin:/usr/local/bin:/usr/sbin:/usr/bin:/sbin:/bin".to_string(),
        );
        Self { env_vars }
    }
}

impl Image for Auth {
    fn name(&self) -> &str {
        NAME
    }

    fn tag(&self) -> &str {
        TAG
    }

    fn ready_conditions(&self) -> Vec<WaitFor> {
        vec![]
    }

    fn env_vars(
        &self,
    ) -> impl IntoIterator<Item = (impl Into<Cow<'_, str>>, impl Into<Cow<'_, str>>)> {
        &self.env_vars
    }

    fn cmd(&self) -> impl IntoIterator<Item = impl Into<Cow<'_, str>>> {
        vec!["auth"]
    }
    #[allow(unused_variables)]
    fn exec_after_start(
        &self,
        cs: ContainerState,
    ) -> Result<Vec<ExecCommand>, TestcontainersError> {
        Ok(vec![])
    }
}
