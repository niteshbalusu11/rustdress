use hyper::{Body, Response, StatusCode};
use ring::digest;
use serde::{Deserialize, Serialize};
use serde_json::{json, to_vec};
use sha2::{Digest, Sha256};
use urlencoding::decode;

use super::utils::{get_identifiers, get_nostr_keys};

#[derive(Debug, Deserialize, Serialize)]
pub struct ZapRequest {
    content: String,
    created_at: u64,
    id: String,
    kind: u64,
    pubkey: String,
    sig: String,
    tags: Vec<Vec<String>>,
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
                    if a < 1000 || a > 100000000 {
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
            if comment.len() > 280 {
                return Err("CommentCannotBeBlankOrGreaterThan50Characters".to_string());
            }

            return Ok(comment);
        }

        None => Ok("".to_string()),
    }
}

pub fn parse_nostr_query(key: Option<(String, String)>) -> Result<String, String> {
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

                    if calculate_id(&p) != p.id {
                        return Err("InvalidZapRequestId".to_string());
                    }

                    if get_nostr_keys().is_err() {
                        return Err("FailedToGetNostrKeys".to_string());
                    }

                    return Ok(decoded_url.to_string());
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

pub fn event_commitment(ev: &ZapRequest) -> Vec<u8> {
    let pubkey = ev.pubkey.clone();
    let created_at = ev.created_at;
    let kind = ev.kind;
    let tags = ev.tags.clone();
    let content = ev.content.clone();

    let commitment = json!([0, pubkey, created_at, kind, tags, content]);
    to_vec(&commitment).unwrap()
}

pub fn calculate_id(ev: &ZapRequest) -> String {
    let commitment = event_commitment(&ev);
    let hash = Sha256::digest(&commitment);
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
        "commentAllowed": 50,
        "maxSendable": 100000000,
        "metadata": metadata,
        "minSendable": 1000,
        "tag": "payRequest",
        "status": "OK",
    });

    let pubkey = match get_nostr_keys() {
        Ok((_, key)) => key,
        Err(_) => "".to_string(),
    };

    if !pubkey.is_empty() {
        response_body["allowNostr"] = serde_json::Value::Bool(true);
        response_body["nostrPubkey"] = serde_json::Value::String(pubkey);
    }

    let response_body_string =
        serde_json::to_string(&response_body).expect("Failed to serialize response body to JSON");

    return response_body_string;
}

pub fn get_digest(nostr: String) -> Vec<u8> {
    let (domain, username) = get_identifiers();

    let identifier = format!("{}@{}", username, domain);

    let default_metadata = serde_json::to_string(&[
        ["text/identifier", &identifier],
        ["text/plain", &format!("Paying satoshis to {}", identifier)],
    ])
    .expect("Failed to serialize metadata");

    let metadata = if nostr.is_empty() {
        default_metadata
    } else {
        nostr
    };

    let digest = digest::digest(&digest::SHA256, metadata.as_bytes())
        .as_ref()
        .to_vec();

    return digest;
}
