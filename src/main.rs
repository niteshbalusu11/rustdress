use credentials::get_lnd::{get_lnd, test_invoice};
use server::{constants::EnvVariables, start_server::start_server, utils::nip05_broadcast};
mod server;

mod credentials;
use dotenv::dotenv;

#[tokio::main]
async fn main() {
    dotenv().ok();

    // Check if username and domain exist

    let domain = std::env::var(EnvVariables::DOMAIN).expect("ExpectedDomainNameAsEnvVariable");
    let username = std::env::var(EnvVariables::USERNAME).expect("ExpectedUserNameAsEnvVariable");

    let lnd = get_lnd().await;
    test_invoice(lnd).await;

    nip05_broadcast(domain, username).await;

    start_server().await;
}
