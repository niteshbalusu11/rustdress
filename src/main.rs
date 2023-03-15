use credentials::get_lnd::{get_lnd, test_invoice};
use server::{start_server::start_server, utils::nip05_broadcast};
mod server;

mod credentials;
use dotenv::dotenv;

#[tokio::main]
async fn main() {
    dotenv().ok();

    // Check if username and domain exist

    let domain = std::env::var("DOMAIN");
    let username = std::env::var("USERNAME");

    if domain.is_err() || domain.as_ref().unwrap().is_empty() {
        panic!("ExpectedDomainNameAsEnvVariable");
    }

    if username.is_err() || username.as_ref().unwrap().is_empty() {
        panic!("ExpectedUserNameAsEnvVariable");
    }

    let lnd = get_lnd().await;
    test_invoice(lnd).await;

    nip05_broadcast(domain.unwrap(), username.unwrap()).await;

    start_server().await;
}
