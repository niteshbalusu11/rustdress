use std::time::{SystemTime, UNIX_EPOCH};

use bech32::{encode, ToBase32, Variant};
use dotenv::dotenv;
use lnd_grpc_rust::{
    invoicesrpc::SubscribeSingleInvoiceRequest,
    lnrpc::{invoice::InvoiceState, Invoice},
    LndClient,
};
use secp256k1::{PublicKey, Secp256k1, SecretKey};
use serde_json::json;

use crate::{
    credentials::get_lnd::get_lnd,
    server::{
        constants::CONSTANTS,
        parsing_functions::calculate_id,
        publish_to_relay::{publish, sign_message},
    },
};

use super::{
    constants::EnvVariables,
    parsing_functions::{convert_key, ZapRequest},
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

pub fn buffer_as_hex(bytes: Vec<u8>) -> String {
    let hex_str = bytes
        .iter()
        .map(|b| format!("{:02x}", b))
        .collect::<String>();

    return hex_str;
}

pub fn get_nostr_keys() -> Result<(String, String), String> {
    dotenv::dotenv().ok();

    let privkey = match std::env::var(EnvVariables::NOSTR_PRIVATE_KEY) {
        Ok(key) => convert_key(&key),
        Err(_) => return Err("NostrPrivateKeyIsUndefined".to_string()),
    };

    let privkey_bytes = hex::decode(&privkey).map_err(|_| "InvalidPrivateKey".to_string())?;
    let pubkey_bytes = private_key_to_public_key(&privkey_bytes);
    let pubkey_hex = hex::encode(&pubkey_bytes);

    Ok((privkey, pubkey_hex))
}

fn private_key_to_public_key(privkey: &[u8]) -> Vec<u8> {
    let secp = Secp256k1::new();
    let secret_key = SecretKey::from_slice(privkey).unwrap();
    let (public_key, _) = PublicKey::from_secret_key(&secp, &secret_key).x_only_public_key();

    let serialized = public_key.serialize().to_vec();

    return serialized;
}

pub async fn create_invoice(
    digest: Vec<u8>,
    comment: String,
    amount: i64,
    nostr_query: Result<ZapRequest, String>,
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
    zap_request: ZapRequest,
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
            let relays = CONSTANTS.relays;

            let content = format!(
                "{{\"name\": \"{}\", \"nip05\": \"{}@{}\"}}",
                username, username, domain
            );

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
}
