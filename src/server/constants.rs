use serde::{Deserialize, Serialize};

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
    max_sendamount: 10000000000,
    min_sendamount: 1000,
};

#[derive(Serialize, Deserialize)]
pub(crate) struct Nip05EventDetails {
    pub content: String,
    pub created_at: u64,
    pub id: String,
    pub kind: u16,
    pub pubkey: String,
    pub tags: Vec<String>,
    pub sig: String,
}
