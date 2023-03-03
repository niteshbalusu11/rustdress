use crate::{credentials::get_lnd::get_lnd, server::utils::bech32_encode};
use http::uri::Uri;
use hyper::{http, Body, Request, Response, StatusCode};
use lnd_grpc_rust::lnrpc::Invoice;
use ring::digest;
use serde_json::json;

use super::utils::{add_hop_hints, get_identifiers};

pub async fn handle_request(req: Request<Body>) -> Result<Response<Body>, hyper::Error> {
    match (req.method(), req.uri().path()) {
        (&hyper::Method::GET, "/") => {
            return handle_default_path();
        }

        (&hyper::Method::GET, path) if path.starts_with("/.well-known/lnurlp/") => {
            return handle_invoice_path(path, req.uri()).await
        }
        // Return 404 Not Found for any other requests
        _ => return handle_unknown_path(),
    }
}

fn handle_default_path() -> Result<Response<Body>, hyper::Error> {
    let (domain, username) = get_identifiers();

    let lnurl_url = "https://".to_owned() + &domain + "/.well-known/lnurlp/" + &username;
    let encoded = bech32_encode("lnurl".to_string(), lnurl_url.clone());

    match encoded {
        Ok(s) => {
            let response_body = json!({ "lnurl": s, "decoded_url": lnurl_url.clone(), "info": {"title": "RustDress: Lightning Address Personal Server", "source": "https://github.com/niteshbalusu11/rustdress"}  });
            let response_body_string = serde_json::to_string(&response_body)
                .expect("Failed to serialize response body to JSON");

            return handle_ok_request(response_body_string);
        }
        Err(_) => handle_bad_request("Failed To Encode Lnurl"),
    }
}

fn handle_unknown_path() -> Result<Response<Body>, hyper::Error> {
    let response_body = json!({ "status": "ERROR", "reason": "Invalid Path"});

    let response_body_string =
        serde_json::to_string(&response_body).expect("Failed to serialize response body to JSON");

    return handle_ok_request(response_body_string);
}

async fn handle_invoice_path(path: &str, uri: &Uri) -> Result<Response<Body>, hyper::Error> {
    let (domain, username) = get_identifiers();

    let identifier = format!("{}@{}", username, domain);

    let metadata = serde_json::to_string(&[
        ["text/identifier", &identifier],
        ["text/plain", &format!("Paying satoshis to {}", identifier)],
    ])
    .expect("Failed to serialize metadata");

    let digest = digest::digest(&digest::SHA256, metadata.as_bytes())
        .as_ref()
        .to_vec();

    let lnurl_url = "https://".to_owned() + &domain + "/.well-known/lnurlp/" + username.as_str();

    let response_body = json!({
        "callback": lnurl_url,
        "commentAllowed": 50,
        "maxSendable": 100000000,
        "metadata": metadata,
        "minSendable": 1000,
        "tag": "payRequest",
        "status": "OK",
    });
    let response_body_string =
        serde_json::to_string(&response_body).expect("Failed to serialize response body to JSON");

    let username = path.rsplitn(2, '/').next();
    match username {
        Some(name) if !name.is_empty() => {
            if let Some(query_str) = uri.query() {
                let query_pairs: Vec<(String, String)> = query_str
                    .split('&')
                    .map(|kv| {
                        let mut iter = kv.split('=');
                        let key = iter.next().unwrap().to_string();
                        let value = iter.next().unwrap_or("").to_string();
                        (key, value)
                    })
                    .collect();

                let amount_key = find_key("amount", &query_pairs);
                let comment_key = find_key("comment", &query_pairs);
                let amount = match parse_amount_key(amount_key.cloned()) {
                    Ok(a) => a,
                    Err(_) => {
                        return handle_bad_request("UnableToParseAmount");
                    }
                };

                if amount == 0 {
                    return handle_ok_request(response_body_string);
                }

                let comment = match parse_comment_key(comment_key.cloned()) {
                    Ok(c) => c,
                    Err(_) => {
                        return handle_bad_request("FailedToParseComments");
                    }
                };

                let mut lnd = get_lnd().await;

                let result = lnd
                    .lightning()
                    .add_invoice(Invoice {
                        description_hash: digest,
                        expiry: 300,
                        memo: comment,
                        private: add_hop_hints(),
                        value_msat: amount,
                        ..Default::default()
                    })
                    .await
                    .expect("FailedToAuthenticateToLnd");
                let pr = result.into_inner().payment_request;

                let success_response_body = json!({
                    "disposable": false,
                    "pr": pr,
                    "routes": [],
                    "status": "OK",
                    "successAction": { "tag": "message", "message": "Payment received!" },
                });

                let success_response_body_string = serde_json::to_string(&success_response_body)
                    .expect("Failed to serialize response body to JSON");

                return handle_ok_request(success_response_body_string);
            } else {
                return handle_ok_request(response_body_string);
            }
        }
        _ => return handle_bad_request("Username Not Found"),
    }
}

fn find_key<'a>(key: &'a str, vector: &'a [(String, String)]) -> Option<&'a (String, String)> {
    vector.iter().find(|(k, _)| *k == key)
}

fn handle_bad_request(reason: &str) -> Result<Response<Body>, hyper::Error> {
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

fn handle_ok_request(body: String) -> Result<Response<Body>, hyper::Error> {
    let resp = Response::builder()
        .status(StatusCode::OK)
        .header("content-type", "application/json")
        .header("Access-Control-Allow-Origin", "*")
        .body(Body::from(body))
        .unwrap();
    Ok(resp)
}

fn parse_amount_key(key: Option<(String, String)>) -> Result<i64, String> {
    match key {
        Some((_, amount)) => {
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

fn parse_comment_key(key: Option<(String, String)>) -> Result<String, String> {
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
