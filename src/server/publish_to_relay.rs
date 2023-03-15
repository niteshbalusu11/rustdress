use crate::server::{parsing_functions::get_tags, utils::get_nostr_keys};
use futures::{future::join_all, SinkExt, StreamExt};
use secp256k1::{KeyPair, Message, PublicKey, Secp256k1, SecretKey};
use serde_json::json;
use std::vec;
use tokio_tungstenite::connect_async;
use tungstenite::Message as SocketMessage;

use super::parsing_functions::{calculate_id, ZapRequest};

pub fn sign_message(privkey: String, message: &str) -> String {
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
    zap_request_json: ZapRequest,
    comment: &str,
    payment_request: String,
    preimage: Vec<u8>,
    settle_date: i64,
) {
    let decoded_preimage = hex::encode(preimage);
    let (privkey, pubkey) = get_nostr_keys().unwrap();
    let zap_request_string = serde_json::to_string::<ZapRequest>(&zap_request_json)
        .expect("FailedToParseZapRequestForPublishingToRelays");

    let relays = get_tags(&zap_request_json.tags, "relays")
        .expect("FailedToParseE-TagsForPublishingToRelays");

    let get_etags =
        get_tags(&zap_request_json.tags, "e").expect("FailedToParseP-TagsForPublishingToRelays");

    let get_ptags =
        get_tags(&zap_request_json.tags, "p").expect("FailedToParseP-TagsForPublishingToRelays");

    let ptags = vec!["p", &get_ptags[0]];
    let etags = vec!["e", &get_etags[0]];

    let bolt11 = vec!["bolt11", &payment_request];

    let description = vec!["description", &zap_request_string];

    let payment_secret = vec!["preimage", &decoded_preimage];

    let content = if comment.is_empty() {
        zap_request_json.content
    } else {
        comment.to_string()
    };

    let mut tags = Vec::new();
    tags.push(ptags);
    tags.push(etags);
    tags.push(bolt11);
    tags.push(payment_secret);
    tags.push(description);

    let id = calculate_id(json!([0, pubkey, settle_date, 9735, tags, content,]));
    let sig = sign_message(privkey, &id);

    let zap_note = json!([
        "EVENT",
        {
        "id": id,
        "pubkey": pubkey,
        "created_at": settle_date,
        "kind": 9735,
        "tags": tags,
        "content": content,
        "sig": sig
    }]);

    let publish_message =
        serde_json::to_string(&zap_note).expect("Failed to serialize response body to JSON");

    tokio::spawn(async move {
        publish(relays, publish_message).await;
    });
}

pub async fn publish(relays: Vec<String>, publish_message: String) {
    let mut futures = vec![];

    for relay in relays {
        let (host, port) = match relay.split_once("://") {
            Some((_, addr)) => match addr.split_once(":") {
                Some((host, port)) => (host, port),
                None => (addr, "443"),
            },
            None => continue,
        };

        let uri = format!("wss://{}:{}/", host, port);
        let future = send_message(uri, publish_message.clone());
        futures.push(future);
    }

    let results = join_all(futures).await;

    // Handle any errors that occurred
    for (_index, _result) in results.into_iter().enumerate() {}
}

async fn send_message(uri: String, message: String) -> Result<(), ()> {
    let (mut socket, _) = match connect_async(uri.clone()).await {
        Ok((websocket_stream, res)) => (websocket_stream, res),
        Err(err) => {
            println!("Failed to connect to URI {:?}: {:?}", uri, err);
            return Err(());
        }
    };

    match socket.send(SocketMessage::Text(message)).await {
        Ok(_) => println!("Sent message to {:?}", uri),
        Err(_) => {
            println!("Failed to send message to {:?}", uri);
            return Err(());
        }
    }

    while let Some(result) = socket.next().await {
        match result {
            Ok(msg) => {
                match msg {
                    SocketMessage::Text(text) => {
                        println!("Received message from {:?}: {:?}", uri, text);
                        break; // exit the loop after receiving a message
                    }
                    _ => {
                        println!("Received non-text message from {:?}: {:?}", uri, msg);
                    }
                }
            }
            Err(err) => {
                println!("Error receiving message from {:?}: {:?}", uri, err);
                return Err(());
            }
        }
    }

    match socket.close(None).await {
        Ok(_) => {
            println!("Closed socket connection for {:?}", uri);
            Ok(())
        }
        Err(_) => {
            println!("Failed to close socket connection for {:?}", uri);
            Err(())
        }
    }
}
