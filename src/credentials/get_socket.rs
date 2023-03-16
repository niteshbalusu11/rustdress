use dotenv::dotenv;

use crate::server::constants::EnvVariables;

pub fn get_socket() -> String {
    dotenv().ok();

    let socket = std::env::var(EnvVariables::SOCKET).expect("ExpectedSocketToAuthenticateToLnd");

    return socket;
}
