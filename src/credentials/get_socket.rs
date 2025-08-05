use crate::config::get_config;

pub fn get_socket() -> String {
    let config = get_config();
    config.lnd.socket.clone()
}
