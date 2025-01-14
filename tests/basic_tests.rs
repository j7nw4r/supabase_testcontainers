use std::time::Duration;
use testcontainers::runners::AsyncRunner;
use testcontainers_modules::postgres::Postgres;
use tokio::time::sleep;

use supabase_testcontainers::Auth;

#[tokio::test]
#[ignore]
async fn supabase_auth_default() -> anyhow::Result<()> {
    let postgres_container = Postgres::default().with_host_auth().start().await?;

    let docker_internal_connection_string = format!(
        "postgres://supabase_auth_admin:root@{}:{}/postgres",
        supabase_testcontainers::DOCKER_INTERNAL_HOST,
        postgres_container.get_host_port_ipv4(5432).await?
    );
    let local_host_connection_string = format!(
        "postgres://postgres:postgres@{}:{}/postgres",
        supabase_testcontainers::LOCAL_HOST,
        postgres_container.get_host_port_ipv4(5432).await?
    );

    let supabase_auth_container = Auth::new(&docker_internal_connection_string)
        // .with_mount()
        .init_db_schema(&local_host_connection_string)
        .await?
        .start()
        .await?;
    sleep(Duration::from_secs(5)).await;

    // // Print stdout
    // let Ok(supabase_auth_container_stdout_byte_vec) = supabase_auth_container.stdout_to_vec().await else {
    //     panic!("could not get supabase auth container stdout");
    // };
    // let stdout_string = String::from_utf8_lossy(supabase_auth_container_stdout_byte_vec.as_ref()).to_string();
    // println!("supabase auth container stdout: {}", stdout_string);

    // Print stderr
    let Ok(supabase_auth_container_stderr_byte_vec) = supabase_auth_container.stderr_to_vec().await
    else {
        panic!("could not get supabase auth container stderr");
    };
    let stderr_string =
        String::from_utf8_lossy(supabase_auth_container_stderr_byte_vec.as_ref()).to_string();
    eprintln!("supabase auth container stderr: {}", stderr_string);

    Ok(())
}
