use dotenv::dotenv;
use std::fs;

use crate::server::utils::buffer_as_hex;

pub fn get_cert() -> String {
    dotenv().ok();

    // Check if all env variables are present.
    let cert_path = std::env::var("CERT_PATH");
    let cert_hex = std::env::var("CERT_HEX");

    // Check if macaroon_path and macaroon_hex are both empty or undefined.
    if (cert_path.is_err() || cert_path.as_ref().unwrap().is_empty())
        && (cert_hex.is_err() || cert_hex.as_ref().unwrap().is_empty())
    {
        panic!("ExpectedEitherTlsCertPathOrTlsCertHexToAuthenticateToLnd");
    }

    if !cert_path.is_err() && !cert_path.as_ref().unwrap().is_empty() {
        let cert_bytes = fs::read(cert_path.unwrap()).expect("FailedToReadTlsCertFile");

        return buffer_as_hex(cert_bytes);
    } else {
        return cert_hex.unwrap();
    }
}
