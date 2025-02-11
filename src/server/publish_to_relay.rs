use crate::server::{parsing_functions::get_tags, utils::get_nostr_keys};
use futures::{future::join_all, SinkExt};
use rusted_nostr_tools::event_methods::{get_event_hash, sign_event, SignedEvent, UnsignedEvent};
use serde_json::json;
use std::vec;
use tokio_tungstenite::connect_async;
use tracing::{debug, error, info, warn};
use tungstenite::Message as SocketMessage;

use super::utils::get_relays;

pub fn publish_zap_to_relays(
    zap_request_json: SignedEvent,
    comment: &str,
    payment_request: String,
    preimage: Vec<u8>,
    settle_date: i64,
) {
    info!(target: "server::publish", "Publishing zap to relays");
    debug!(target: "server::publish", "Zap request content: {}", zap_request_json.content);

    let decoded_preimage = hex::encode(preimage);
    let (privkey, pubkey) = match get_nostr_keys() {
        Ok(keys) => keys,
        Err(e) => {
            error!(target: "server::publish", "Failed to get nostr keys: {}", e);
            return;
        }
    };

    let zap_request_string = match serde_json::to_string::<SignedEvent>(&zap_request_json) {
        Ok(str) => str,
        Err(e) => {
            error!(target: "server::publish", "Failed to parse zap request for publishing to relays: {}", e);
            return;
        }
    };

    let relays = match get_tags(&zap_request_json.tags, "relays") {
        Some(r) => r,
        _ => {
            error!(target: "server::publish", "Failed to parse relay tags for publishing");
            return;
        }
    };

    let combined_relays = get_relays(Some(relays));
    debug!(target: "server::publish", "Publishing to {} relays", combined_relays.len());

    let get_etags = match get_tags(&zap_request_json.tags, "e") {
        Some(tags) => tags,
        _ => {
            error!(target: "server::publish", "Failed to parse e-tags for publishing");
            return;
        }
    };

    let get_ptags = match get_tags(&zap_request_json.tags, "p") {
        Some(tags) => tags,
        _ => {
            error!(target: "server::publish", "Failed to parse p-tags for publishing");
            return;
        }
    };

    let ptags = vec!["p".to_string(), get_ptags[0].clone()];
    let etags = vec!["e".to_string(), get_etags[0].clone()];
    let bolt11 = vec!["bolt11".to_string(), payment_request.clone()];
    let description = vec!["description".to_string(), zap_request_string.clone()];
    let payment_secret = vec!["preimage".to_string(), decoded_preimage.clone()];

    let content = if comment.is_empty() {
        debug!(target: "server::publish", "Using zap request content as no comment provided");
        zap_request_json.content
    } else {
        debug!(target: "server::publish", "Using provided comment: {}", comment);
        comment.to_string()
    };

    let tags = vec![ptags, etags, bolt11, payment_secret, description];

    let event: UnsignedEvent = UnsignedEvent {
        pubkey: pubkey.clone(),
        created_at: settle_date,
        kind: 9735,
        tags: tags.clone(),
        content: content.clone(),
    };

    let id = match get_event_hash(&event) {
        Ok(id) => id,
        Err(e) => {
            error!(target: "server::publish", "Failed to calculate event hash for publishing: {}", e);
            return;
        }
    };

    let signature = match sign_event(&event, &privkey) {
        Ok(sig) => sig,
        Err(e) => {
            error!(target: "server::publish", "Failed to sign event for publishing: {}", e);
            return;
        }
    };

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

    let publish_message = match serde_json::to_string(&zap_note) {
        Ok(msg) => msg,
        Err(e) => {
            error!(target: "server::publish", "Failed to serialize zap note: {}", e);
            return;
        }
    };

    debug!(target: "server::publish", "Spawning task to publish zap note");
    tokio::spawn(async move {
        publish(combined_relays, publish_message).await;
    });
}

pub async fn publish(relays: Vec<String>, publish_message: String) {
    info!(target: "server::publish", "Starting publish to {} relays", relays.len());
    let mut futures = vec![];

    for relay in &relays {
        let (host, port) = match relay.split_once("://") {
            Some((_, addr)) => match addr.split_once(':') {
                Some((host, port)) => (host, port),
                None => (addr, "443"),
            },
            None => {
                warn!(target: "server::publish", "Invalid relay URL format: {}", relay);
                continue;
            }
        };

        let uri = format!("wss://{}:{}/", host, port);
        debug!(target: "server::publish", "Adding relay to publish queue: {}", uri);
        let future = send_message(uri, publish_message.clone());
        futures.push(future);
    }

    let results = join_all(futures).await;
    let mut success_count = 0;
    let mut failure_count = 0;

    for result in results {
        match result {
            Ok(_) => success_count += 1,
            Err(_) => failure_count += 1,
        }
    }

    info!(
        target: "server::publish",
        "Publish complete. Successful: {}, Failed: {}",
        success_count,
        failure_count
    );
}

async fn send_message(uri: String, message: String) -> Result<(), ()> {
    debug!(target: "server::publish", "Attempting to connect to relay: {}", uri);
    let (mut socket, _) = match connect_async(uri.clone()).await {
        Ok((websocket_stream, res)) => {
            debug!(target: "server::publish", "Successfully connected to {}", uri);
            (websocket_stream, res)
        }
        Err(err) => {
            error!(target: "server::publish", "Failed to connect to {}: {}", uri, err);
            return Err(());
        }
    };

    match socket.send(SocketMessage::Text(message)).await {
        Ok(_) => {
            info!(target: "server::publish", "Successfully sent message to {}", uri);
        }
        Err(e) => {
            error!(target: "server::publish", "Failed to send message to {}: {}", uri, e);
            return Err(());
        }
    }

    match socket.close(None).await {
        Ok(_) => {
            debug!(target: "server::publish", "Successfully closed connection to {}", uri);
            Ok(())
        }
        Err(e) => {
            warn!(target: "server::publish", "Failed to close socket connection for {}: {}", uri, e);
            Err(())
        }
    }
}
