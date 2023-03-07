use crate::server::{parsing_functions::get_tags, utils::get_nostr_keys};
use futures_util::sink::SinkExt;
use secp256k1::{KeyPair, Message, PublicKey, Secp256k1, SecretKey};
use serde::{Deserialize, Serialize};
use serde_json::json;
use tokio_tungstenite::connect_async;
use tungstenite::Message as SocketMessage;

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

fn sign_message(privkey: String, message: &str) -> String {
    let secp = Secp256k1::new();
    let secret_key =
        SecretKey::from_slice(&hex::decode(privkey).expect("FailedToDecodeHexPrivateKey"))
            .expect("32 bytes, within curve order");
    let (xpub, _) = PublicKey::from_secret_key(&secp, &secret_key).x_only_public_key();
    let pair = KeyPair::from_seckey_slice(&secp, &secret_key.secret_bytes())
        .expect("Failed to generate keypair from secret key");

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
    let zap_request_json = serde_json::from_str::<ZapRequest2>(&zap_request)
        .expect("FailedToParseZapRequestForPublishingToRelays");
    let relays = get_tags(&zap_request_json.tags, "relays")
        .expect("FailedToParseE-TagsForPublishingToRelays");

    let ptags =
        get_tags(&zap_request_json.tags, "p").expect("FailedToParseP-TagsForPublishingToRelays");
    let etags =
        get_tags(&zap_request_json.tags, "e").expect("FailedToParseE-TagsForPublishingToRelays");

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

    let publish_message =
        serde_json::to_string(&zap_note).expect("Failed to serialize response body to JSON");

    tokio::spawn(async move {
        publish(relays, publish_message).await;
    });
}

async fn publish(relays: Vec<String>, publish_message: String) {
    println!("{:?}", relays);
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
            .send(SocketMessage::Text(publish_message.to_string()))
            .await
        {
            println!("Failed to send message to relay {:?}: {:?}", relay, err);
            continue;
        }

        println!("Sent message to {:?}", relay);
    }
}
