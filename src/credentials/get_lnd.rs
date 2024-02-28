use crate::credentials::{get_cert::get_cert, get_macaroon::get_macaroon, get_socket::get_socket};
use lnd_grpc_rust::{lnrpc::Invoice, LndClient};

pub async fn get_lnd() -> LndClient {
    let cert = get_cert();
    let macaroon = get_macaroon();
    let socket = get_socket();

    let client = lnd_grpc_rust::connect(cert, macaroon, socket)
        .await
        .expect("FailedToAuthenticateToLnd");

    return client;
}

pub async fn test_invoice(mut client: LndClient) -> Result<(), anyhow::Error> {
    client
        .lightning()
        .add_invoice(Invoice {
            value: 5,
            expiry: 100,
            ..Default::default()
        })
        .await?;

    Ok(())
}
