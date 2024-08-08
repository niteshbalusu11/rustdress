use dotenv::dotenv;

use crate::server::constants::EnvVariables;

pub fn get_socket() -> String {
    dotenv().ok();

    std::env::var(EnvVariables::SOCKET).expect("ExpectedSocketToAuthenticateToLnd")
}
