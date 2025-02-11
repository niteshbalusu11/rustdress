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
use tracing::{debug, error, info, warn};

use crate::{
    credentials::get_lnd::get_lnd,
    server::{constants::CONSTANTS, publish_to_relay::publish},
};

use super::{
    constants::{EnvVariables, Nip05EventDetails},
    parsing_functions::convert_key,
    publish_to_relay::publish_zap_to_relays,
};

pub fn get_identifiers() -> (String, String) {
    debug!(target: "server::utils", "Loading environment variables for identifiers");
    dotenv().ok();

    let domain = match std::env::var(EnvVariables::DOMAIN) {
        Ok(domain) => {
            debug!(target: "server::utils", "Loaded domain: {}", domain);
            domain
        }
        Err(e) => {
            error!(target: "server::utils", "Failed to load domain: {}", e);
            String::new()
        }
    };

    let username = match std::env::var(EnvVariables::USERNAME) {
        Ok(username) => {
            debug!(target: "server::utils", "Loaded username: {}", username);
            username
        }
        Err(e) => {
            error!(target: "server::utils", "Failed to load username: {}", e);
            String::new()
        }
    };

    (domain, username)
}

pub fn bech32_encode(prefix: String, data: String) -> Result<String, bech32::Error> {
    debug!(target: "server::utils", "Encoding data to bech32. Prefix: {}", prefix);
    let base32_data = data.to_base32();

    match encode(&prefix, base32_data, Variant::Bech32) {
        Ok(encoded) => {
            debug!(target: "server::utils", "Successfully encoded data to bech32");
            Ok(encoded)
        }
        Err(e) => {
            error!(target: "server::utils", "Failed to encode data to bech32: {}", e);
            Err(e)
        }
    }
}

pub fn add_hop_hints() -> bool {
    debug!(target: "server::utils", "Checking if hop hints should be included");
    let is_add_hints = std::env::var(EnvVariables::INCLUDE_HOP_HINTS);

    match is_add_hints {
        Ok(add) => match add.as_str() {
            "true" => {
                debug!(target: "server::utils", "Hop hints will be included");
                true
            }
            "false" => {
                debug!(target: "server::utils", "Hop hints will not be included");
                false
            }
            _ => {
                warn!(target: "server::utils", "Invalid value for INCLUDE_HOP_HINTS: {}, defaulting to false", add);
                false
            }
        },
        Err(e) => {
            debug!(target: "server::utils", "INCLUDE_HOP_HINTS not set ({}), defaulting to false", e);
            false
        }
    }
}

pub fn get_nostr_keys() -> Result<(String, String), String> {
    debug!(target: "server::utils", "Loading nostr keys");
    dotenv::dotenv().ok();

    let privkey = match std::env::var(EnvVariables::NOSTR_PRIVATE_KEY) {
        Ok(key) => {
            let converted = convert_key(&key);
            debug!(target: "server::utils", "Successfully loaded and converted private key");
            converted
        }
        Err(e) => {
            error!(target: "server::utils", "Failed to load nostr private key: {}", e);
            return Err("NostrPrivateKeyIsUndefined".to_string());
        }
    };

    let binding = GeneratePublicKey::new(&privkey);
    let pubkey_hex = binding.hex_public_key();
    debug!(target: "server::utils", "Generated public key from private key");

    Ok((privkey, pubkey_hex.to_string()))
}

pub async fn create_invoice(
    digest: Vec<u8>,
    comment: String,
    amount: i64,
    nostr_query: Result<SignedEvent, String>,
) -> String {
    info!(target: "server::utils", "Creating invoice for amount: {}, comment: {}", amount, comment);
    let mut lnd = get_lnd().await;

    let result = match lnd
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
    {
        Ok(result) => result,
        Err(e) => {
            error!(target: "server::utils", "Failed to create invoice: {}", e);
            panic!("FailedToCreateInvoice");
        }
    };

    let invoice_result = result.into_inner();
    info!(target: "server::utils", "Created invoice with payment request: {}", invoice_result.payment_request);

    if nostr_query.is_ok() {
        let r_hash = invoice_result.r_hash;
        let zap_request = nostr_query.unwrap();
        let comment_clone = comment.clone();
        debug!(target: "server::utils", "Starting invoice watcher for zap request");
        tokio::spawn(async move {
            watch_invoice(zap_request, lnd, &r_hash, &comment_clone).await;
        });
    }
    invoice_result.payment_request
}

async fn watch_invoice(zap_request: SignedEvent, mut lnd: LndClient, r_hash: &[u8], comment: &str) {
    debug!(target: "server::utils", "Starting to watch invoice for payment");
    let mut invoice_subscription = match lnd
        .invoices()
        .subscribe_single_invoice(SubscribeSingleInvoiceRequest {
            r_hash: r_hash.to_vec(),
        })
        .await
    {
        Ok(sub) => {
            debug!(target: "server::utils", "Successfully subscribed to invoice updates");
            sub.into_inner()
        }
        Err(e) => {
            error!(target: "server::utils", "Failed to subscribe to invoice: {}", e);
            return;
        }
    };

    while let Some(invoice) = match invoice_subscription.message().await {
        Ok(msg) => msg,
        Err(e) => {
            error!(target: "server::utils", "Failed to receive invoice subscription info: {}", e);
            return;
        }
    } {
        if let Some(state) = InvoiceState::from_i32(invoice.state) {
            debug!(target: "server::utils", "Invoice state update: {:?}", state);
            // If this invoice was Settled we can do something with it
            if state == InvoiceState::Settled {
                info!(target: "server::utils", "Invoice settled, publishing zap to relays");
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
                debug!(target: "server::utils", "Invoice settled, stopping watch");
                break;
            }
        }
    }
}

pub async fn nip05_broadcast(domain: String, username: String) {
    info!(target: "server::utils", "Broadcasting NIP-05 verification for {}@{}", username, domain);
    match get_nostr_keys() {
        Ok((privkey, pubkey)) => {
            let relays = get_relays(None);
            debug!(target: "server::utils", "Using {} relays for broadcast", relays.len());

            let content = format!(
                "{{\"name\": \"{}\", \"nip05\": \"{}@{}\"}}",
                username, username, domain
            );

            let current_time = SystemTime::now().duration_since(UNIX_EPOCH).unwrap();
            let timestamp = current_time.as_secs();

            let event = UnsignedEvent {
                content: content.clone(),
                created_at: timestamp as i64,
                kind: 0,
                tags: [].to_vec(),
                pubkey: pubkey.clone(),
            };

            let id = match get_event_hash(&event) {
                Ok(id) => id,
                Err(e) => {
                    error!(target: "server::utils", "Failed to calculate event hash: {}", e);
                    return;
                }
            };

            let signature = match sign_event(&event, &privkey) {
                Ok(sig) => sig,
                Err(e) => {
                    error!(target: "server::utils", "Failed to sign event: {}", e);
                    return;
                }
            };

            let nip05_event_details = Nip05EventDetails {
                content,
                created_at: timestamp,
                id,
                kind: 0,
                pubkey: pubkey.clone(),
                tags: vec![],
                sig: signature.sig,
            };

            let event = ("EVENT".to_string(), nip05_event_details);

            let publish_message = match serde_json::to_string(&event) {
                Ok(msg) => msg,
                Err(e) => {
                    error!(target: "server::utils", "Failed to serialize event: {}", e);
                    return;
                }
            };

            debug!(target: "server::utils", "Spawning task to publish NIP-05 verification");
            tokio::spawn(async move {
                publish(relays, publish_message).await;
            });

            info!(target: "server::utils", "NIP-05 broadcast initiated with pubkey: {}", pubkey);
        }
        Err(e) => {
            error!(target: "server::utils", "Failed to get nostr keys: {}", e);
        }
    };
}

pub fn get_relays(relays: Option<Vec<String>>) -> Vec<String> {
    debug!(target: "server::utils", "Getting relay list");
    let arg_relays = relays.unwrap_or_default();
    debug!(target: "server::utils", "Argument relays count: {}", arg_relays.len());

    let env_relays = std::env::var(EnvVariables::RELAYS);
    let mut env_relays_vec: Vec<String> = vec![];

    if let Ok(ref r) = env_relays {
        env_relays_vec = r.split(',').map(|s| s.to_string()).collect();
        debug!(target: "server::utils", "Environment relays count: {}", env_relays_vec.len());
    } else {
        debug!(target: "server::utils", "No environment relays configured");
    }

    let default_relays: Vec<String> = CONSTANTS.relays.iter().map(|s| s.to_string()).collect();
    debug!(target: "server::utils", "Default relays count: {}", default_relays.len());

    // Create a HashSet from both vectors to remove duplicates.
    let mut combined_relays: HashSet<String> = env_relays_vec.into_iter().collect();
    combined_relays.extend(default_relays);
    combined_relays.extend(arg_relays);

    let unique_relays: Vec<String> = combined_relays.into_iter().collect();
    info!(target: "server::utils", "Final unique relays count: {}", unique_relays.len());

    unique_relays
}
