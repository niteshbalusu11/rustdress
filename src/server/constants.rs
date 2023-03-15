pub struct Constants {
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
};
