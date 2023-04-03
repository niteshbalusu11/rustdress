use std::ffi::OsStr;

pub struct Constants {
    pub max_comment_length: usize,
    pub max_sendamount: i64,
    pub min_sendamount: i64,
    pub relays: [&'static str; 14],
}

pub const CONSTANTS: Constants = Constants {
    relays: [
        "wss://nostr.foundrydigital.com",
        "wss://eden.nostr.land",
        "wss://relay.damus.io",
        "wss://relay.snort.social",
        "wss://nos.lol",
        "wss://relay.current.fyi",
        "wss://relay.nostr.info",
        "wss://nostr.zebedee.cloud",
        "wss://nostr.fmt.wiz.biz",
        "wss://relay.nostr.bg",
        "wss://nostr.mom",
        "wss://nostr.bitcoiner.social",
        "wss://nostr.oxtr.dev",
        "wss://no.str.cr",
    ],
    max_comment_length: 280,
    max_sendamount: 100000000,
    min_sendamount: 1000,
};

#[allow(non_camel_case_types)]
pub enum EnvVariables {
    USERNAME,
    DOMAIN,
    CERT_PATH,
    MACAROON_PATH,
    CERT_HEX,
    MACAROON_HEX,
    SOCKET,
    HOST,
    PORT,
    INCLUDE_HOP_HINTS,
    NOSTR_PRIVATE_KEY,
    NIP_05_PUBKEY,
    RELAYS,
}

impl AsRef<OsStr> for EnvVariables {
    fn as_ref(&self) -> &OsStr {
        match self {
            EnvVariables::DOMAIN => OsStr::new("DOMAIN"),
            EnvVariables::PORT => OsStr::new("PORT"),
            EnvVariables::USERNAME => OsStr::new("USERNAME"),
            EnvVariables::CERT_PATH => OsStr::new("CERT_PATH"),
            EnvVariables::MACAROON_PATH => OsStr::new("MACAROON_PATH"),
            EnvVariables::CERT_HEX => OsStr::new("CERT_HEX"),
            EnvVariables::MACAROON_HEX => OsStr::new("MACAROON_HEX"),
            EnvVariables::SOCKET => OsStr::new("SOCKET"),
            EnvVariables::HOST => OsStr::new("HOST"),
            EnvVariables::INCLUDE_HOP_HINTS => OsStr::new("INCLUDE_HOP_HINTS"),
            EnvVariables::NOSTR_PRIVATE_KEY => OsStr::new("NOSTR_PRIVATE_KEY"),
            EnvVariables::NIP_05_PUBKEY => OsStr::new("NIP_05_PUBKEY"),
            EnvVariables::RELAYS => OsStr::new("RELAYS"),
        }
    }
}
