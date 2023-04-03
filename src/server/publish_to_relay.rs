use crate::server::{parsing_functions::get_tags, utils::get_nostr_keys};
use futures::{future::join_all, SinkExt};
use rusted_nostr_tools::event_methods::{get_event_hash, sign_event, SignedEvent, UnsignedEvent};
use serde_json::json;
use std::vec;
use tokio_tungstenite::connect_async;
use tungstenite::Message as SocketMessage;

use super::utils::get_relays;

pub fn publish_zap_to_relays(
    zap_request_json: SignedEvent,
    comment: &str,
    payment_request: String,
    preimage: Vec<u8>,
    settle_date: i64,
) {
    let decoded_preimage = hex::encode(preimage);
    let (privkey, pubkey) = get_nostr_keys().unwrap();
    let zap_request_string = serde_json::to_string::<SignedEvent>(&zap_request_json)
        .expect("FailedToParseZapRequestForPublishingToRelays");

    let relays = get_tags(&zap_request_json.tags, "relays")
        .expect("FailedToParseE-TagsForPublishingToRelays");

    let combined_relays = get_relays(Some(relays));

    let get_etags =
        get_tags(&zap_request_json.tags, "e").expect("FailedToParseP-TagsForPublishingToRelays");

    let get_ptags =
        get_tags(&zap_request_json.tags, "p").expect("FailedToParseP-TagsForPublishingToRelays");

    let ptags = vec!["p".to_string(), get_ptags[0].clone()];
    let etags = vec!["e".to_string(), get_etags[0].clone()];

    let bolt11 = vec!["bolt11".to_string(), payment_request.clone()];

    let description = vec!["description".to_string(), zap_request_string.clone()];

    let payment_secret = vec!["preimage".to_string(), decoded_preimage.clone()];

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

    let event: UnsignedEvent = UnsignedEvent {
        pubkey: pubkey.clone(),
        created_at: settle_date,
        kind: 9735,
        tags: tags.clone(),
        content: content.clone(),
    };

    let id = get_event_hash(&event).expect("FailedToCalculateEventHashForPublishingToRelays");

    let signature = sign_event(&event, &privkey).expect("FailedToSignEventForPublishingToRelays");

    let zap_note = json!([
        "EVENT",
        {
        "id": id,
        "pubkey": pubkey,
        "created_at": settle_date,
        "kind": 9735,
        "tags": tags,
        "content": content,
        "sig": signature.sig
    }]);

    let publish_message =
        serde_json::to_string(&zap_note).expect("Failed to serialize response body to JSON");

    tokio::spawn(async move {
        publish(combined_relays, publish_message).await;
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
        Err(_) => println!("Failed to send message to {:?}", uri),
    }

    match socket.close(None).await {
        Ok(_) => Ok(()),
        Err(_) => {
            println!("Failed to close socket connection for {:?}", uri);
            Err(())
        }
    }
}
