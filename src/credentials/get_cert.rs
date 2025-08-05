use crate::config::get_config;
use std::fs;

pub fn get_cert() -> String {
    let config = get_config();
    let lnd_config = &config.lnd;

    let cert_path = &lnd_config.cert_path;
    let cert_hex = &lnd_config.cert_hex;

    if cert_path.is_none() && cert_hex.is_none() {
        panic!("ExpectedEitherTlsCertPathOrTlsCertHexToAuthenticateToLnd");
    }

    if let Some(path) = cert_path {
        if !path.is_empty() {
            let cert_bytes = fs::read(path).expect("FailedToReadTlsCertFile");
            return hex::encode(cert_bytes);
        }
    }

    if let Some(hex) = cert_hex {
        if !hex.is_empty() {
            return hex.to_string();
        }
    }

    panic!("ExpectedEitherTlsCertPathOrTlsCertHexToAuthenticateToLnd");
}
