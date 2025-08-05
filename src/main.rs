use credentials::get_lnd::{get_lnd, test_invoice};
use server::{start_server::start_server, utils::nip05_broadcast};
mod config;
mod server;

mod credentials;
use crate::config::get_config;
use tracing::{info, Level};
use tracing_subscriber::{EnvFilter, FmtSubscriber};

#[tokio::main]
async fn main() -> Result<(), anyhow::Error> {
    // Initialize logging
    let _subscriber = FmtSubscriber::builder()
        .with_env_filter(
            EnvFilter::from_default_env()
                .add_directive(Level::DEBUG.into())
                .add_directive("rustdress=debug".parse().unwrap()),
        )
        .with_thread_ids(true)
        .with_target(true)
        .with_file(true)
        .with_line_number(true)
        .with_thread_names(true)
        .pretty()
        .init();

    info!("Starting Rustdress application");

    let config = get_config();
    let domain = config.domain.clone();

    info!("Connecting to LND node");
    let lnd = get_lnd().await;

    info!("Testing invoice generation");
    test_invoice(lnd).await?;

    info!("Broadcasting NIP-05 verification");
    for user in &config.users {
        nip05_broadcast(domain.clone(), user.username.clone()).await;
    }

    info!("Starting server");
    start_server().await?;

    Ok(())
}
