use hyper::{Body, Response, StatusCode};
use rusted_nostr_tools::{
    event_methods::{get_event_hash, SignedEvent, UnsignedEvent},
    ConvertKey,
};
use serde_json::json;
use sha2::{Digest, Sha256};
use tracing::{debug, error, warn};
use urlencoding::decode;

use crate::{config::get_config, server::constants::CONSTANTS};

use super::utils::{get_identifiers, get_nostr_keys};

pub fn find_key<'a>(key: &'a str, vector: &'a [(String, String)]) -> Option<&'a (String, String)> {
    debug!(target: "server::parsing", "Searching for key: {} in query parameters", key);
    vector.iter().find(|(k, _)| *k == key)
}

pub fn handle_bad_request(reason: &str) -> Result<Response<Body>, hyper::Error> {
    warn!(target: "server::parsing", "Handling bad request: {}", reason);
    let response_body = json!({ "status": "ERROR", "reason": reason});

    let response_body_string = match serde_json::to_string(&response_body) {
        Ok(body) => body,
        Err(e) => {
            error!(target: "server::parsing", "Failed to serialize error response: {}", e);
            return handle_bad_request("Internal Server Error");
        }
    };

    let resp = Response::builder()
        .status(StatusCode::BAD_REQUEST)
        .header("Access-Control-Allow-Origin", "*")
        .body(Body::from(response_body_string))
        .unwrap();
    Ok(resp)
}

pub fn handle_ok_request(body: String) -> Result<Response<Body>, hyper::Error> {
    debug!(target: "server::parsing", "Handling successful request");
    let resp = Response::builder()
        .status(StatusCode::OK)
        .header("content-type", "application/json")
        .header("Access-Control-Allow-Origin", "*")
        .body(Body::from(body))
        .unwrap();
    Ok(resp)
}

pub fn parse_amount_query(key: Option<(String, String)>) -> Result<i64, String> {
    match key {
        Some((_, amount)) => {
            if amount.is_empty() {
                debug!(target: "server::parsing", "Empty amount provided, returning 0");
                return Ok(0);
            }

            let amount_str = amount.clone();
            let amount = amount.parse::<i64>();

            match amount {
                Ok(a) => {
                    if !(CONSTANTS.min_sendamount..=CONSTANTS.max_sendamount).contains(&a) {
                        warn!(target: "server::parsing", "Amount {} is out of range [{}, {}]", 
                            a, CONSTANTS.min_sendamount, CONSTANTS.max_sendamount);
                        return Err("AmountOutOfRange".to_string());
                    }

                    debug!(target: "server::parsing", "Successfully parsed amount: {}", a);
                    Ok(a)
                }

                Err(e) => {
                    error!(target: "server::parsing", "Failed to parse amount '{}': {}", amount_str, e);
                    Err("FailedToParseAmount".to_string())
                }
            }
        }
        None => {
            debug!(target: "server::parsing", "No amount provided, returning 0");
            Ok(0)
        }
    }
}

pub fn parse_comment_query(key: Option<(String, String)>) -> Result<String, String> {
    match key {
        Some((_, comment)) => {
            if comment.len() > CONSTANTS.max_comment_length {
                warn!(target: "server::parsing", "Comment length {} exceeds maximum {}", 
                    comment.len(), CONSTANTS.max_comment_length);
                return Err("CommentCannotBeBlankOrGreaterThan50Characters".to_string());
            }

            debug!(target: "server::parsing", "Successfully parsed comment: {}", comment);
            Ok(comment)
        }

        None => {
            debug!(target: "server::parsing", "No comment provided");
            Ok("".to_string())
        }
    }
}

pub fn parse_name_query(key: Option<(String, String)>) -> Result<String, String> {
    match key {
        Some((_, name)) => {
            debug!(target: "server::parsing", "Successfully parsed name: {}", name);
            Ok(name)
        }

        None => {
            warn!(target: "server::parsing", "No name provided in query");
            Err("".to_string())
        }
    }
}

pub fn parse_nostr_query(key: Option<(String, String)>) -> Result<SignedEvent, String> {
    match key {
        Some((_, nostr)) => {
            debug!(target: "server::parsing", "Attempting to parse nostr query");
            let decoded_url = match decode(&nostr) {
                Ok(url) => url,
                Err(e) => {
                    error!(target: "server::parsing", "Failed to decode nostr query string: {}", e);
                    return Err("FailedToDecodeNostrQueryString".to_string());
                }
            };

            match serde_json::from_str::<SignedEvent>(&decoded_url) {
                Ok(p) => {
                    if p.kind != 9734 {
                        warn!(target: "server::parsing", "Invalid zap kind: {}", p.kind);
                        return Err("InvalidZapKind".to_string());
                    }

                    if p.tags.is_empty() {
                        warn!(target: "server::parsing", "Missing tags in zap request");
                        return Err("MissingTagKeyInZapRequest".to_string());
                    }

                    let tags = p.tags.clone();
                    let ptags = get_tags(&tags, "p");

                    if ptags.is_none() {
                        warn!(target: "server::parsing", "Missing p-tags in zap request");
                        return Err("MissingP-TagsInZapRequest".to_string());
                    }

                    if ptags.is_some() && ptags.unwrap().len() >= 2 {
                        warn!(target: "server::parsing", "Multiple p-tags found in zap request");
                        return Err("MultipleP-TagsArePresentInTheZapRequest".to_string());
                    }

                    let etags = get_tags(&tags, "e");

                    if etags.is_none() {
                        warn!(target: "server::parsing", "Missing e-tags in zap request");
                        return Err("MissingE-TagsInZapRequest".to_string());
                    }

                    let relaytags = get_tags(&tags, "relays");

                    if relaytags.is_none() {
                        warn!(target: "server::parsing", "Missing relay tags in zap request");
                        return Err("MissingRelaysInZapRequest".to_string());
                    }

                    let event = UnsignedEvent {
                        content: p.content.clone(),
                        created_at: p.created_at,
                        kind: p.kind,
                        tags: p.tags.clone(),
                        pubkey: p.pubkey.clone(),
                    };

                    let id = match get_event_hash(&event) {
                        Ok(id) => id,
                        Err(e) => {
                            error!(target: "server::parsing", "Failed to get event hash: {}", e);
                            return Err("FailedToGetEventHash".to_string());
                        }
                    };

                    if id != p.id {
                        warn!(target: "server::parsing", "Invalid zap request ID. Expected: {}, Got: {}", id, p.id);
                        return Err("InvalidZapRequestId".to_string());
                    }

                    if let Err(e) = get_nostr_keys() {
                        error!(target: "server::parsing", "Failed to get nostr keys: {}", e);
                        return Err("FailedToGetNostrKeys".to_string());
                    }

                    debug!(target: "server::parsing", "Successfully parsed nostr query");
                    Ok(p)
                }

                Err(e) => {
                    error!(target: "server::parsing", "Failed to parse nostr query: {}", e);
                    Err("FailedToParseNostrQuery".to_string())
                }
            }
        }

        None => {
            debug!(target: "server::parsing", "No nostr query provided");
            Err("".to_string())
        }
    }
}

pub fn get_tags(tags: &[Vec<String>], key: &str) -> Option<Vec<String>> {
    debug!(target: "server::parsing", "Getting tags for key: {}", key);
    let mut values = Vec::new();

    for tag in tags.iter() {
        if tag[0] == key {
            if key == "relays" {
                for value in tag.iter().skip(1) {
                    values.push(value.clone());
                }
            } else {
                values.push(tag[1].clone());
            }
        }
    }

    if values.is_empty() {
        debug!(target: "server::parsing", "No tags found for key: {}", key);
        None
    } else {
        debug!(target: "server::parsing", "Found {} tags for key: {}", values.len(), key);
        Some(values)
    }
}

pub fn handle_response_body() -> String {
    debug!(target: "server::parsing", "Generating response body");
    let (domain, username) = get_identifiers();

    let identifier = format!("{}@{}", username, domain);
    debug!(target: "server::parsing", "Using identifier: {}", identifier);

    let metadata = match serde_json::to_string(&[
        ["text/identifier", &identifier],
        ["text/plain", &format!("Paying satoshis to {}", identifier)],
    ]) {
        Ok(metadata) => metadata,
        Err(e) => {
            error!(target: "server::parsing", "Failed to serialize metadata: {}", e);
            return "".to_string();
        }
    };

    let lnurl_url = "https://".to_owned() + &domain + "/.well-known/lnurlp/" + username.as_str();

    let config = get_config();
    let max_sendable = config.max_sendable.unwrap_or(CONSTANTS.max_sendamount);

    let mut response_body = json!({
        "callback": lnurl_url,
        "commentAllowed": CONSTANTS.max_comment_length,
        "maxSendable": max_sendable,
        "metadata": metadata,
        "minSendable": CONSTANTS.min_sendamount,
        "tag": "payRequest",
        "status": "OK",
    });

    let pubkey = match get_nostr_keys() {
        Ok((_, key)) => key,
        Err(e) => {
            warn!(target: "server::parsing", "Failed to get nostr keys: {}", e);
            "".to_string()
        }
    };

    if !pubkey.is_empty() {
        debug!(target: "server::parsing", "Adding nostr pubkey to response: {}", pubkey);
        response_body["allowsNostr"] = serde_json::Value::Bool(true);
        response_body["nostrPubkey"] = serde_json::Value::String(pubkey);
    }

    match serde_json::to_string(&response_body) {
        Ok(body) => body,
        Err(e) => {
            error!(target: "server::parsing", "Failed to serialize response body: {}", e);
            "".to_string()
        }
    }
}

pub fn get_digest(nostr: Option<&SignedEvent>) -> Vec<u8> {
    debug!(target: "server::parsing", "Calculating digest");
    let mut hasher = Sha256::new();

    let (domain, username) = get_identifiers();
    let identifier = format!("{}@{}", username, domain);

    let default_metadata = match serde_json::to_string(&[
        ["text/identifier", &identifier],
        ["text/plain", &format!("Paying satoshis to {}", identifier)],
    ]) {
        Ok(metadata) => metadata,
        Err(e) => {
            error!(target: "server::parsing", "Failed to serialize default metadata: {}", e);
            "".to_string()
        }
    };

    match nostr {
        Some(event) => {
            debug!(target: "server::parsing", "Using nostr event for digest calculation");
            hasher.update(event.id.as_bytes());
            hasher.finalize().to_vec()
        }
        None => {
            debug!(target: "server::parsing", "Using default metadata for digest calculation");
            hasher.update(default_metadata.as_bytes());
            hasher.finalize().to_vec()
        }
    }
}

pub fn convert_key(key: &str) -> String {
    match ConvertKey::to_hex(key) {
        Ok(key) => key,
        Err(_) => key.to_string(),
    }
}
