use hyper::{Body, Response, StatusCode};
use serde::{Deserialize, Serialize};
use serde_json::{json, Value};
use sha2::{Digest, Sha256};
use urlencoding::decode;

use crate::server::constants::CONSTANTS;

use super::utils::{get_identifiers, get_nostr_keys};

#[derive(Debug, Deserialize, Serialize)]
pub struct ZapRequest {
    pub content: String,
    pub created_at: u64,
    pub id: String,
    pub kind: u64,
    pub pubkey: String,
    pub sig: String,
    pub tags: Vec<Vec<String>>,
}

pub fn find_key<'a>(key: &'a str, vector: &'a [(String, String)]) -> Option<&'a (String, String)> {
    vector.iter().find(|(k, _)| *k == key)
}

pub fn handle_bad_request(reason: &str) -> Result<Response<Body>, hyper::Error> {
    let response_body = json!({ "status": "ERROR", "reason": reason});

    let response_body_string =
        serde_json::to_string(&response_body).expect("Failed to serialize response body to JSON");

    let resp = Response::builder()
        .status(StatusCode::BAD_REQUEST)
        .header("Access-Control-Allow-Origin", "*")
        .body(Body::from(response_body_string))
        .unwrap();
    Ok(resp)
}

pub fn handle_ok_request(body: String) -> Result<Response<Body>, hyper::Error> {
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
                return Ok(0);
            }

            let amount = amount.parse::<i64>();

            match amount {
                Ok(a) => {
                    if a < CONSTANTS.min_sendamount || a > CONSTANTS.max_sendamount {
                        return Err("AmountOutOfRange".to_string());
                    }

                    Ok(a)
                }

                _ => Err("FailedToParseAmount".to_string()),
            }
        }
        None => Ok(0),
    }
}

pub fn parse_comment_query(key: Option<(String, String)>) -> Result<String, String> {
    match key {
        Some((_, comment)) => {
            if comment.len() > CONSTANTS.max_comment_length {
                return Err("CommentCannotBeBlankOrGreaterThan50Characters".to_string());
            }

            return Ok(comment);
        }

        None => Ok("".to_string()),
    }
}

pub fn parse_name_query(key: Option<(String, String)>) -> Result<String, String> {
    match key {
        Some((_, comment)) => {
            return Ok(comment);
        }

        None => Err("".to_string()),
    }
}

pub fn parse_nostr_query(key: Option<(String, String)>) -> Result<ZapRequest, String> {
    match key {
        Some((_, nostr)) => {
            let decoded_url = match decode(&nostr) {
                Ok(url) => url,
                Err(_) => return Err("FailedToDecodeNostrQueryString".to_string()),
            };

            match serde_json::from_str::<ZapRequest>(&decoded_url) {
                Ok(p) => {
                    if p.kind != 9734 {
                        return Err("InvalidZapKind".to_string());
                    }

                    if p.tags.is_empty() {
                        return Err("MissingTagKeyInZapRequest".to_string());
                    }

                    let tags = p.tags.clone();
                    if tags.is_empty() {
                        return Err("EmptyTagsInZapRequest".to_string());
                    }

                    let ptags = get_tags(&tags, "p");

                    if ptags.is_none() {
                        return Err("MissingP-TagsInZapRequest".to_string());
                    }

                    if ptags.is_some() {
                        if ptags.unwrap().len() >= 2 {
                            return Err("MultipleP-TagsArePresentInTheZapRequest".to_string());
                        }
                    }

                    let etags = get_tags(&tags, "e");

                    if etags.is_none() {
                        return Err("MissingE-TagsInZapRequest".to_string());
                    }

                    let relaytags = get_tags(&tags, "relays");

                    if relaytags.is_none() {
                        return Err("MissingRelaysInZapRequest".to_string());
                    }

                    let id = calculate_id(json!([
                        0,
                        p.pubkey,
                        p.created_at,
                        p.kind,
                        p.tags,
                        p.content,
                    ]));

                    if id != p.id {
                        return Err("InvalidZapRequestId".to_string());
                    }

                    if get_nostr_keys().is_err() {
                        return Err("FailedToGetNostrKeys".to_string());
                    }

                    return Ok(p);
                }

                Err(_) => return Err("FailedToParseNostrQuery".to_string()),
            };
        }

        _ => Err("".to_string()),
    }
}

pub fn get_tags(tags: &Vec<Vec<String>>, key: &str) -> Option<Vec<String>> {
    let mut values = Vec::new();

    for tag in tags.iter() {
        if tag[0] == key {
            if key == "relays" {
                for i in 1..tag.len() {
                    values.push(tag[i].clone());
                }
            } else {
                values.push(tag[1].clone());
            }
        }
    }

    if values.is_empty() {
        None
    } else {
        Some(values)
    }
}

pub fn calculate_id(commitment: Value) -> String {
    let commitment_string =
        serde_json::to_string(&commitment).expect("Failed to serialize response body to JSON");

    let mut hasher = Sha256::new();
    hasher.update(commitment_string.as_bytes());
    let hash = hasher.finalize();
    hex::encode(hash)
}

pub fn handle_response_body() -> String {
    let (domain, username) = get_identifiers();

    let identifier = format!("{}@{}", username, domain);

    let metadata = serde_json::to_string(&[
        ["text/identifier", &identifier],
        ["text/plain", &format!("Paying satoshis to {}", identifier)],
    ])
    .expect("Failed to serialize metadata");

    let lnurl_url = "https://".to_owned() + &domain + "/.well-known/lnurlp/" + username.as_str();

    let mut response_body = json!({
        "callback": lnurl_url,
        "commentAllowed": CONSTANTS.max_comment_length,
        "maxSendable": CONSTANTS.max_sendamount,
        "metadata": metadata,
        "minSendable": CONSTANTS.min_sendamount,
        "tag": "payRequest",
        "status": "OK",
    });

    let pubkey = match get_nostr_keys() {
        Ok((_, key)) => key,
        Err(_) => "".to_string(),
    };

    if !pubkey.is_empty() {
        response_body["allowsNostr"] = serde_json::Value::Bool(true);
        response_body["nostrPubkey"] = serde_json::Value::String(pubkey);
    }

    let response_body_string =
        serde_json::to_string(&response_body).expect("Failed to serialize response body to JSON");

    return response_body_string;
}

pub fn get_digest(nostr: Option<&ZapRequest>) -> Vec<u8> {
    let mut hasher = Sha256::new();

    let (domain, username) = get_identifiers();

    let identifier = format!("{}@{}", username, domain);

    let default_metadata = serde_json::to_string(&[
        ["text/identifier", &identifier],
        ["text/plain", &format!("Paying satoshis to {}", identifier)],
    ])
    .expect("Failed to serialize metadata");

    let metadata = if nostr.is_none() {
        default_metadata
    } else {
        serde_json::to_string(&Some(nostr.unwrap())).unwrap_or(default_metadata)
    };

    hasher.update(metadata.as_bytes());

    hasher.finalize().to_vec()
}
