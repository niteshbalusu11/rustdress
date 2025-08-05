use crate::config::get_config;
use std::fs;

pub fn get_macaroon() -> String {
    let config = get_config();
    let lnd_config = &config.lnd;

    let macaroon_path = &lnd_config.macaroon_path;
    let macaroon_hex = &lnd_config.macaroon_hex;

    if macaroon_path.is_none() && macaroon_hex.is_none() {
        panic!("ExpectedEitherMacaroonPathOrMacaroonHexToAuthenticateToLnd");
    }

    if let Some(path) = macaroon_path {
        if !path.is_empty() {
            let mac_bytes = fs::read(path).expect("FailedToReadMacaroonFile");
            return hex::encode(mac_bytes);
        }
    }

    if let Some(hex) = macaroon_hex {
        if !hex.is_empty() {
            return hex.to_string();
        }
    }

    panic!("ExpectedEitherMacaroonPathOrMacaroonHexToAuthenticateToLnd");
}
