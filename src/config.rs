use lazy_static::lazy_static;
use once_cell::sync::OnceCell;
use serde::Deserialize;
use std::env;
use std::fs;
use tracing::{debug, error, info};

static CONFIG: OnceCell<Config> = OnceCell::new();

lazy_static! {
    static ref CONFIG_PATH: String = {
        let args: Vec<String> = env::args().collect();
        let config_path = if let Some(pos) = args.iter().position(|s| s == "--config") {
            args.get(pos + 1).cloned()
        } else {
            None
        };
        match config_path {
            Some(p) => p,
            None => {
                let home_dir = dirs::home_dir().expect("Failed to get home directory");
                home_dir
                    .join(".rustdress")
                    .join("rustdress.toml")
                    .to_str()
                    .unwrap()
                    .to_string()
            }
        }
    };
}

#[derive(Deserialize, Debug, Clone)]
pub struct User {
    pub username: String,
    pub pubkey: String,
}

#[derive(Deserialize, Debug, Clone)]
pub struct Lnd {
    pub cert_path: Option<String>,
    pub cert_hex: Option<String>,
    pub macaroon_path: Option<String>,
    pub macaroon_hex: Option<String>,
    pub socket: String,
}

#[derive(Deserialize, Debug, Clone)]
pub struct Server {
    pub host: String,
    pub port: u16,
}

#[derive(Deserialize, Debug, Clone)]
pub struct Nostr {
    pub private_key: String,
    pub relays: Option<Vec<String>>,
}

#[derive(Deserialize, Debug, Clone)]
pub struct Config {
    pub domain: String,
    pub max_sendable: Option<i64>,
    pub include_hop_hints: Option<bool>,
    pub users: Vec<User>,
    pub lnd: Lnd,
    pub server: Server,
    pub nostr: Nostr,
}

pub fn get_config() -> &'static Config {
    CONFIG.get_or_init(|| {
        info!(target: "config", "Attempting to load and parse {}", *CONFIG_PATH);
        let contents = match fs::read_to_string(&*CONFIG_PATH) {
            Ok(c) => {
                debug!(target: "config", "Successfully read {}", *CONFIG_PATH);
                c
            }
            Err(e) => {
                error!(target: "config", "Failed to read {}: {}", *CONFIG_PATH, e);
                panic!("Failed to read config file");
            }
        };

        match toml::from_str(&contents) {
            Ok(config) => {
                info!(target: "config", "Successfully parsed {}", *CONFIG_PATH);
                config
            }
            Err(e) => {
                error!(target: "config", "Failed to parse {}: {}", *CONFIG_PATH, e);
                panic!("Failed to parse config file");
            }
        }
    })
}
