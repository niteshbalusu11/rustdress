use credentials::get_lnd::{get_lnd, test_invoice};
use server::{constants::EnvVariables, start_server::start_server, utils::nip05_broadcast};
mod server;

mod credentials;
use dotenv::dotenv;
use tracing::{error, info, Level};
use tracing_subscriber::{EnvFilter, FmtSubscriber};

#[tokio::main]
async fn main() -> Result<(), anyhow::Error> {
    // Initialize logging
    let _subscriber = FmtSubscriber::builder()
        .with_env_filter(
            EnvFilter::from_default_env()
                .add_directive(Level::INFO.into())
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

    dotenv().ok();
    info!("Loaded environment variables");

    // Check if username and domain exist
    let domain = std::env::var(EnvVariables::DOMAIN).map_err(|e| {
        error!("Failed to get DOMAIN environment variable: {}", e);
        e
    })?;
    let username = std::env::var(EnvVariables::USERNAME).map_err(|e| {
        error!("Failed to get USERNAME environment variable: {}", e);
        e
    })?;

    info!("Connecting to LND node");
    let lnd = get_lnd().await;

    info!("Testing invoice generation");
    test_invoice(lnd).await?;

    info!("Broadcasting NIP-05 verification");
    nip05_broadcast(domain, username).await;

    info!("Starting server");
    start_server().await?;

    Ok(())
}
