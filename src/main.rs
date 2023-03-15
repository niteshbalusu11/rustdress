use std::time::{SystemTime, UNIX_EPOCH};

use credentials::get_lnd::{get_lnd, test_invoice};
use serde_json::json;
use server::{publish_to_relay::publish, start_server::start_server, utils::get_nostr_keys};
mod server;

mod credentials;
use dotenv::dotenv;

use crate::server::{parsing_functions::calculate_id, publish_to_relay::sign_message};

#[tokio::main]
async fn main() {
    dotenv().ok();

    // Check if username and domain exist

    let domain = std::env::var("DOMAIN");
    let username = std::env::var("USERNAME");

    if domain.is_err() || domain.as_ref().unwrap().is_empty() {
        panic!("ExpectedDomainNameAsEnvVariable");
    }

    if username.is_err() || username.as_ref().unwrap().is_empty() {
        panic!("ExpectedUserNameAsEnvVariable");
    }

    let lnd = get_lnd().await;
    test_invoice(lnd).await;

    match get_nostr_keys() {
        Ok((privkey, pubkey)) => {
            let relays = vec![
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
            ];

            let content = format!(
                "{{\"name\": \"{}\", \"nip05\": \"{}@{}\"}}",
                username.as_ref().unwrap(),
                username.as_ref().unwrap(),
                domain.as_ref().unwrap()
            );

            println!("{}", content);

            let current_time = SystemTime::now().duration_since(UNIX_EPOCH).unwrap();
            let timestamp = current_time.as_secs();

            let id = calculate_id(json!([0, pubkey, timestamp, 0, [], content]));

            let nip05_json = json!([
                "EVENT",
                {
                    "content": content,
                    "created_at": timestamp,
                    "id": id,
                    "kind": 0,
                    "pubkey": pubkey,
                    "tags": [],
                    "sig": sign_message(privkey, &id)
                },
            ]);

            let relay_string: Vec<String> = relays.iter().map(|s| s.to_string()).collect();

            let publish_message = serde_json::to_string(&nip05_json)
                .expect("Failed to serialize response body to JSON");

            tokio::spawn(async move {
                publish(relay_string, publish_message).await;
            });

            pubkey
        }
        Err(_) => "".to_string(),
    };

    start_server().await;

    println!("Starting server on port 3000");
}
