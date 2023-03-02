use dotenv::dotenv;

pub fn get_socket() -> String {
    dotenv().ok();

    let socket = std::env::var("SOCKET").expect("ExpectedSocketToAuthenticateToLnd");

    return socket;
}
