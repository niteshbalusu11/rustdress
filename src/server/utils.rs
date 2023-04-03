use std::{
    collections::HashSet,
    time::{SystemTime, UNIX_EPOCH},
};

use bech32::{encode, ToBase32, Variant};
use dotenv::dotenv;
use lnd_grpc_rust::{
    invoicesrpc::SubscribeSingleInvoiceRequest,
    lnrpc::{invoice::InvoiceState, Invoice},
    LndClient,
};
use rusted_nostr_tools::{
    event_methods::{get_event_hash, sign_event, SignedEvent, UnsignedEvent},
    GeneratePublicKey,
};
use serde_json::json;

use crate::{
    credentials::get_lnd::get_lnd,
    server::{constants::CONSTANTS, publish_to_relay::publish},
};

use super::{
    constants::EnvVariables, parsing_functions::convert_key,
    publish_to_relay::publish_zap_to_relays,
};

pub fn get_identifiers() -> (String, String) {
    dotenv().ok();

    let domain = std::env::var(EnvVariables::DOMAIN).unwrap();
    let username = std::env::var(EnvVariables::USERNAME).unwrap();

    return (domain, username);
}

pub fn bech32_encode(prefix: String, data: String) -> Result<String, bech32::Error> {
    let base32_data = data.to_base32();

    let encoded = encode(&prefix, base32_data, Variant::Bech32);

    match encoded {
        Ok(_) => encoded,
        Err(_) => panic!("FailedToEncodeToBech32"),
    }
}

pub fn add_hop_hints() -> bool {
    let is_add_hints = std::env::var(EnvVariables::INCLUDE_HOP_HINTS);

    match is_add_hints {
        Ok(add) => {
            if add == "true" {
                return true;
            }

            if add == "false" {
                return false;
            }

            false
        }

        Err(_) => false,
    }
}

pub fn get_nostr_keys() -> Result<(String, String), String> {
    dotenv::dotenv().ok();

    let privkey = match std::env::var(EnvVariables::NOSTR_PRIVATE_KEY) {
        Ok(key) => convert_key(&key),
        Err(_) => return Err("NostrPrivateKeyIsUndefined".to_string()),
    };

    let binding = GeneratePublicKey::new(&privkey);
    let pubkey_hex = binding.hex_public_key();

    Ok((privkey, pubkey_hex.to_string()))
}

pub async fn create_invoice(
    digest: Vec<u8>,
    comment: String,
    amount: i64,
    nostr_query: Result<SignedEvent, String>,
) -> String {
    let mut lnd = get_lnd().await;

    let result = lnd
        .lightning()
        .add_invoice(Invoice {
            description_hash: digest,
            expiry: 300,
            memo: comment.clone(),
            private: add_hop_hints(),
            value_msat: amount,
            ..Default::default()
        })
        .await
        .expect("FailedToCreateInvoice");
    let invoice_result = result.into_inner();

    println!(
        "returning payment request {:?}",
        invoice_result.payment_request
    );

    if nostr_query.is_ok() {
        let r_hash = invoice_result.r_hash;
        let zap_request = nostr_query.unwrap();
        let comment_clone = comment.clone();
        tokio::spawn(async move {
            watch_invoice(zap_request, lnd, &r_hash, &comment_clone).await;
        });
    }
    invoice_result.payment_request
}

async fn watch_invoice(
    zap_request: SignedEvent,
    mut lnd: LndClient,
    r_hash: &Vec<u8>,
    comment: &str,
) {
    let mut invoice_subscription = lnd
        .invoices()
        .subscribe_single_invoice(SubscribeSingleInvoiceRequest {
            r_hash: r_hash.to_vec(),
        })
        .await
        .expect("FailedToSubscribeToInvoice")
        .into_inner();

    while let Some(invoice) = invoice_subscription
        .message()
        .await
        .expect("FailedToReceiveInvoiceSubscriptionInfo")
    {
        if let Some(state) = InvoiceState::from_i32(invoice.state) {
            // If this invoice was Settled we can do something with it
            if state == InvoiceState::Settled {
                publish_zap_to_relays(
                    zap_request,
                    comment,
                    invoice.payment_request,
                    invoice.r_preimage,
                    invoice.settle_date,
                );
                break;
            }

            if state == InvoiceState::Settled {
                break;
            }
        }
    }
}

pub async fn nip05_broadcast(domain: String, username: String) {
    match get_nostr_keys() {
        Ok((privkey, pubkey)) => {
            let relays = get_relays(None);

            let content = format!(
                "{{\"name\": \"{}\", \"nip05\": \"{}@{}\"}}",
                username, username, domain
            );

            let current_time = SystemTime::now().duration_since(UNIX_EPOCH).unwrap();
            let timestamp = current_time.as_secs();

            let event = UnsignedEvent {
                content: content.clone(),
                created_at: timestamp.clone() as i64,
                kind: 0,
                tags: [].to_vec(),
                pubkey: pubkey.clone(),
            };

            let id = get_event_hash(&event).expect("FailedToCalculateEventHash");
            let signature = sign_event(&event, &privkey).expect("FailedToSignEvent");

            let nip05_json = json!([
                "EVENT",
                {
                    "content": content,
                    "created_at": timestamp,
                    "id": id,
                    "kind": 0,
                    "pubkey": pubkey,
                    "tags": [],
                    "sig": signature.sig,
                },
            ]);

            let publish_message = serde_json::to_string(&nip05_json)
                .expect("Failed to serialize response body to JSON");

            tokio::spawn(async move {
                publish(relays, publish_message).await;
            });

            pubkey
        }
        Err(_) => "".to_string(),
    };
}

pub fn get_relays(relays: Option<Vec<String>>) -> Vec<String> {
    let arg_relays = relays.unwrap_or(vec![]);

    let env_relays = std::env::var(EnvVariables::RELAYS);
    let mut env_relays_vec: Vec<String> = vec![];

    match env_relays {
        Ok(ref r) => {
            env_relays_vec = r.split(',').map(|s| s.to_string()).collect();
        }
        Err(_) => {}
    };

    println!("Relays: {:?}", env_relays_vec);

    let default_relays: Vec<String> = CONSTANTS.relays.iter().map(|s| s.to_string()).collect();

    // Create a HashSet from both vectors to remove duplicates.
    let mut combined_relays: HashSet<String> = env_relays_vec.into_iter().collect();
    combined_relays.extend(default_relays.into_iter());
    combined_relays.extend(arg_relays.into_iter());

    let unique_relays: Vec<String> = combined_relays.into_iter().collect();

    unique_relays
}
