use dotenv::dotenv;
use std::fs;

use crate::server::constants::EnvVariables;

pub fn get_macaroon() -> String {
    dotenv().ok();

    // Check if all env variables are present.
    let macaroon_path = std::env::var(EnvVariables::MACAROON_PATH);
    let macaroon_hex = std::env::var(EnvVariables::MACAROON_HEX);

    // Check if macaroon_path and macaroon_hex are both empty or undefined.
    if (macaroon_path.is_err() || macaroon_path.as_ref().unwrap().is_empty())
        && (macaroon_hex.is_err() || macaroon_hex.as_ref().unwrap().is_empty())
    {
        panic!("ExpectedEitherMacaroonPathOrMacaroonHexToAuthenticateToLnd");
    }

    if macaroon_path.is_ok() && !macaroon_path.as_ref().unwrap().is_empty() {
        let mac_bytes = fs::read(macaroon_path.unwrap()).expect("FailedToReadMacaroonFile");

        hex::encode(mac_bytes)
    } else {
        macaroon_hex.unwrap()
    }
}
