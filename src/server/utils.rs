use bech32::{encode, ToBase32, Variant};
use dotenv::dotenv;
use lnd_grpc_rust::{
    invoicesrpc::SubscribeSingleInvoiceRequest,
    lnrpc::{invoice::InvoiceState, Invoice},
    LndClient,
};
use secp256k1::{Message, PublicKey, Secp256k1, SecretKey};
use serde::{Deserialize, Serialize};
use serde_json::json;

use crate::{credentials::get_lnd::get_lnd, server::parsing_functions::get_tags};

#[derive(Debug, Deserialize, Serialize)]
pub struct ZapRequest2 {
    content: String,
    created_at: u64,
    id: String,
    kind: u64,
    pubkey: String,
    sig: String,
    tags: Vec<Vec<String>>,
}

pub fn get_identifiers() -> (String, String) {
    dotenv().ok();

    let domain = std::env::var("DOMAIN").unwrap();
    let username = std::env::var("USERNAME").unwrap();

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
    let is_add_hints = std::env::var("INCLUDE_HOP_HINTS");

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

    let privkey = match std::env::var("NOSTR_PRIVATE_KEY") {
        Ok(key) => key,
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
    let public_key = PublicKey::from_secret_key(&secp, &secret_key);
    return public_key.serialize_uncompressed().to_vec();
}

pub async fn create_invoice(
    digest: Vec<u8>,
    comment: String,
    amount: i64,
    nostr_query: Result<String, String>,
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

    if nostr_query.is_ok() {
        println!("inside this");
        let r_hash = invoice_result.r_hash;
        let zap_request = nostr_query.unwrap();
        let comment_clone = comment.clone();
        tokio::spawn(async move {
            watch_invoice(zap_request, lnd, &r_hash, &comment_clone).await;
        });
    }
    println!("returning payment request");
    invoice_result.payment_request
}

async fn watch_invoice(zap_request: String, mut lnd: LndClient, r_hash: &Vec<u8>, comment: &str) {
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

fn publish_zap_to_relays(
    zap_request: String,
    comment: &str,
    payment_request: String,
    preimage: Vec<u8>,
    settle_date: i64,
) {
    let decoded_preimage = hex::encode(preimage);
    let (privkey, pubkey) = get_nostr_keys().unwrap();
    let zap_request_json = serde_json::from_str::<ZapRequest2>(&zap_request).unwrap();
    let ptags = get_tags(&zap_request_json.tags, "p").unwrap();
    let etags = get_tags(&zap_request_json.tags, "e").unwrap();
    let mut bolt11 = Vec::new();
    bolt11.push("bolt11".to_string());
    bolt11.push(payment_request);

    let mut description = Vec::new();
    description.push("description".to_string());
    description.push(zap_request);

    let mut payment_secret = Vec::new();
    payment_secret.push("preimage".to_string());
    payment_secret.push(decoded_preimage);

    let content = if comment.is_empty() {
        zap_request_json.content
    } else {
        comment.to_string()
    };

    let sig = sign_message(privkey, &zap_request_json.id);

    let mut zap_note = json!({
        "kind": 9735,
        "pubkey": pubkey,
        "created_at": settle_date,
        "id": zap_request_json.id,
        "tags": [],
        "content": content,
        "sig": sig
    });
    zap_note["tags"].as_array_mut().unwrap().push(ptags.into());
    zap_note["tags"].as_array_mut().unwrap().push(etags.into());
    zap_note["tags"].as_array_mut().unwrap().push(bolt11.into());
    zap_note["tags"]
        .as_array_mut()
        .unwrap()
        .push(payment_secret.into());
    zap_note["tags"]
        .as_array_mut()
        .unwrap()
        .push(description.into());

    let success_response_body_string =
        serde_json::to_string(&zap_note).expect("Failed to serialize response body to JSON");

    println!("{:?}", success_response_body_string);
}

fn sign_message(privkey: String, message: &str) -> String {
    println!("{:?}", message);
    // Step 1: Convert the string to a byte array
    let msg =
        Message::from_slice(&hex::decode(message).expect("UnableToDecodeHexMessageForSigning"))
            .expect("FailedToConvertHexMessageToBytes");

    // Step 2: Load the private key into a SecretKey object
    let secp = Secp256k1::new();
    let secret_key = hex::decode(privkey).expect("FailedToDecodePrivateKeyToBytes");
    let sk = SecretKey::from_slice(&secret_key).expect("FailedToConvertBytesToSecretKeyType");

    // Step 3: Sign the byte array using the SecretKey object
    let sig = secp.sign_ecdsa(&msg, &sk);

    // Step 4: Convert the signature byte array to a hex string
    hex::encode(sig.serialize_compact())
}
