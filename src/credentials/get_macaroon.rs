use dotenv::dotenv;
use std::fs;

use crate::server::utils::buffer_as_hex;

pub fn get_macaroon() -> String {
    dotenv().ok();

    // Check if all env variables are present.
    let macaroon_path = std::env::var("MACAROON_PATH");
    let macaroon_hex = std::env::var("MACAROON_HEX");

    // Check if macaroon_path and macaroon_hex are both empty or undefined.
    if (macaroon_path.is_err() || macaroon_path.as_ref().unwrap().is_empty())
        && (macaroon_hex.is_err() || macaroon_hex.as_ref().unwrap().is_empty())
    {
        panic!("ExpectedEitherMacaroonPathOrMacaroonHexToAuthenticateToLnd");
    }

    if !macaroon_path.is_err() && !macaroon_path.as_ref().unwrap().is_empty() {
        let mac_bytes = fs::read(macaroon_path.unwrap()).expect("FailedToReadMacaroonFile");

        return buffer_as_hex(mac_bytes);
    } else {
        return macaroon_hex.unwrap();
    }
}
