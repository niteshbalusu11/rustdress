use std::vec;

use crate::server::{parsing_functions::get_tags, utils::get_nostr_keys};
use futures_util::sink::SinkExt;
use secp256k1::{KeyPair, Message, PublicKey, Secp256k1, SecretKey};
use serde_json::json;
use tokio_tungstenite::connect_async;
use tungstenite::Message as SocketMessage;

use super::parsing_functions::{calculate_id, ZapRequest};

fn sign_message(privkey: String, message: &str) -> String {
    let secp = Secp256k1::new();
    let secret_key =
        SecretKey::from_slice(&hex::decode(privkey).expect("FailedToDecodeHexPrivateKey"))
            .expect("32 bytes, within curve order");
    let (xpub, _) = PublicKey::from_secret_key(&secp, &secret_key).x_only_public_key();
    let pair = KeyPair::from_seckey_slice(&secp, &secret_key.secret_bytes())
        .expect("Failed to generate keypair from secret key");

    let public_key = PublicKey::from_secret_key(&secp, &secret_key)
        .serialize()
        .to_vec();

    println!(
        "Public Key while signing is: {:?}, xpub while signing is: {:?}",
        hex::encode(&public_key),
        hex::encode(&xpub.serialize())
    );

    let message =
        Message::from_slice(&hex::decode(message).expect("UnableToDecodeHexMessageForSigning"))
            .expect("FailedToConvertHexMessageToBytes");

    let sig = secp.sign_schnorr_no_aux_rand(&message, &pair);

    assert!(secp.verify_schnorr(&sig, &message, &xpub).is_ok());

    return hex::encode(sig.as_ref());
}

pub fn publish_zap_to_relays(
    zap_request: String,
    comment: &str,
    payment_request: String,
    preimage: Vec<u8>,
    settle_date: i64,
) {
    let decoded_preimage = hex::encode(preimage);
    let (privkey, pubkey) = get_nostr_keys().unwrap();
    let zap_request_json = serde_json::from_str::<ZapRequest>(&zap_request)
        .expect("FailedToParseZapRequestForPublishingToRelays");

    let id = calculate_id(&zap_request_json);

    let relays = get_tags(&zap_request_json.tags, "relays")
        .expect("FailedToParseE-TagsForPublishingToRelays");

    let get_etags =
        get_tags(&zap_request_json.tags, "e").expect("FailedToParseP-TagsForPublishingToRelays");

    let get_ptags =
        get_tags(&zap_request_json.tags, "p").expect("FailedToParseP-TagsForPublishingToRelays");

    let ptags = vec!["p", &get_ptags[0]];
    let etags = vec!["e", &get_etags[0]];

    let bolt11 = vec!["bolt11", &payment_request];

    let description = vec!["description", &zap_request];

    let payment_secret = vec!["preimage", &decoded_preimage];

    let content = if comment.is_empty() {
        zap_request_json.content
    } else {
        comment.to_string()
    };

    let sig = sign_message(privkey, &zap_request_json.id);
    let mut tags = Vec::new();
    tags.push(ptags);
    tags.push(etags);
    tags.push(bolt11);
    tags.push(payment_secret);
    tags.push(description);

    let zap_note = json!({
        "id": id,
        "pubkey": pubkey,
        "created_at": settle_date,
        "kind": 9735,
        "tags": tags,
        "content": content,
        "sig": sig
    });

    let publish_message =
        serde_json::to_string(&zap_note).expect("Failed to serialize response body to JSON");

    let publish_relay_event = serde_json::to_string(&vec!["EVENT", &publish_message]).unwrap();

    println!("zap note to be published:  {:?}", publish_relay_event);

    tokio::spawn(async move {
        publish(relays, publish_message).await;
    });
}

async fn publish(relays: Vec<String>, publish_message: String) {
    for relay in relays {
        let (host, port) = match relay.split_once("://") {
            Some((_, addr)) => match addr.split_once(":") {
                Some((host, port)) => {
                    println!("{:?}, {:?}", host, port);
                    (host, port)
                }
                None => (addr, "443"),
            },
            None => continue,
        };
        let uri = format!("wss://{}:{}/", host, port);

        println!("{:?}", uri);

        // Connect to the url and call the closure
        // Connect to the WebSocket URL and send the message
        let (mut websocket, _) = match connect_async(uri).await {
            Ok((websocket, _)) => (websocket, ()),
            Err(err) => {
                println!("Failed to connect to relay {:?}: {:?}", relay, err);
                continue;
            }
        };

        println!("Connected to {:?}", relay);

        // Send the message over the WebSocket connection
        if let Err(err) = websocket
            .send(SocketMessage::Text(publish_message.clone()))
            .await
        {
            println!("Failed to send message to relay {:?}: {:?}", relay, err);
            continue;
        }

        println!("Sent message to {:?}", relay);
    }
}
