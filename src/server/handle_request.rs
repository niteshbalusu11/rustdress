use crate::server::{constants::CONSTANTS, utils::bech32_encode};
use http::uri::Uri;
use hyper::{http, Body, Request, Response};
use serde::{Deserialize, Serialize};
use serde_json::json;

use super::{
    constants::EnvVariables,
    parsing_functions::{
        convert_key, find_key, get_digest, handle_bad_request, handle_ok_request,
        handle_response_body, parse_amount_query, parse_comment_query, parse_name_query,
        parse_nostr_query,
    },
    utils::{create_invoice, get_identifiers},
};

pub async fn handle_request(req: Request<Body>) -> Result<Response<Body>, hyper::Error> {
    match (req.method(), req.uri().path()) {
        (&hyper::Method::GET, "/") => handle_default_path(),

        (&hyper::Method::GET, path) if path.starts_with("/.well-known/lnurlp/") => {
            handle_invoice_path(path, req.uri()).await
        }

        (&hyper::Method::GET, path) if path.starts_with("/.well-known/nostr.json") => {
            handle_nip05_path(req.uri()).await
        }
        // Return 404 Not Found for any other requests
        _ => handle_unknown_path(),
    }
}

#[derive(Serialize, Deserialize)]
struct Info {
    title: String,
    source: String,
}

#[derive(Serialize, Deserialize)]
struct DefaultPathResponse {
    lnurl: String,
    decoded_url: String,
    info: Info,
}

fn handle_default_path() -> Result<Response<Body>, hyper::Error> {
    let (domain, username) = get_identifiers();

    let lnurl_url = "https://".to_owned() + &domain + "/.well-known/lnurlp/" + &username;
    let encoded = bech32_encode("lnurl".to_string(), lnurl_url.clone());

    match encoded {
        Ok(s) => {
            let response_body = DefaultPathResponse {
                lnurl: s,
                decoded_url: lnurl_url.clone(),
                info: Info {
                    title: "RustDress: Lightning Address Personal Server".to_string(),
                    source: "https://github.com/niteshbalusu11/rustdress".to_string(),
                },
            };

            let response_body_string = serde_json::to_string(&response_body)
                .expect("Failed to serialize response body to JSON");

            handle_ok_request(response_body_string)
        }
        Err(_) => handle_bad_request("Failed To Encode Lnurl"),
    }
}

#[derive(Serialize, Deserialize)]
struct UnknownPathResponse {
    status: String,
    reason: String,
}

fn handle_unknown_path() -> Result<Response<Body>, hyper::Error> {
    let response_body = UnknownPathResponse {
        status: "ERROR".to_string(),
        reason: "Invalid Path".to_string(),
    };

    let response_body_string =
        serde_json::to_string(&response_body).expect("Failed to serialize response body to JSON");

    handle_ok_request(response_body_string)
}

#[derive(Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
struct SuccessPathResponse {
    disposable: bool,
    pr: String,
    routes: Vec<String>,
    status: String,
    success_action: SuccessAction,
}

#[derive(Serialize, Deserialize)]
struct SuccessAction {
    tag: String,
    message: String,
}

async fn handle_invoice_path(path: &str, uri: &Uri) -> Result<Response<Body>, hyper::Error> {
    let username = path.rsplit('/').next();
    let response_body_string = handle_response_body();

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
                let nostr_key = find_key("nostr", &query_pairs);

                let parsed_nostr_query = parse_nostr_query(nostr_key.cloned());

                let digest = get_digest(parsed_nostr_query.as_ref().ok());

                let amount = match parse_amount_query(amount_key.cloned()) {
                    Ok(a) => a,
                    Err(_) => {
                        return handle_bad_request("UnableToParseAmount");
                    }
                };

                let comment = match parse_comment_query(comment_key.cloned()) {
                    Ok(c) => c,
                    Err(_) => {
                        return handle_bad_request("FailedToParseComments");
                    }
                };

                if amount == 0 {
                    return handle_ok_request(response_body_string);
                }

                let pr = create_invoice(digest, comment, amount, parsed_nostr_query).await;
                let success_response_body = SuccessPathResponse {
                    disposable: false,
                    pr,
                    routes: vec![],
                    status: "OK".to_string(),
                    success_action: SuccessAction {
                        tag: "message".to_string(),
                        message: "Payment received!".to_string(),
                    },
                };

                let success_response_body_string = serde_json::to_string(&success_response_body)
                    .expect("Failed to serialize response body to JSON");

                return handle_ok_request(success_response_body_string);
            }
            handle_ok_request(response_body_string)
        }
        _ => handle_bad_request("Username Not Found"),
    }
}

async fn handle_nip05_path(uri: &Uri) -> Result<Response<Body>, hyper::Error> {
    let pubkey = match std::env::var(EnvVariables::NIP_05_PUBKEY) {
        Ok(key) => convert_key(&key),
        Err(_) => return handle_bad_request("Failed To Get Nostr Keys"),
    };

    let relays = CONSTANTS.relays;
    let username = std::env::var(EnvVariables::USERNAME).unwrap();

    let default_response_body = json!({
      "names": {
        &username: &pubkey,
      },
      "relays": {
        &pubkey: &relays,
      }
    });

    let default_response_body_string = serde_json::to_string(&default_response_body)
        .expect("Failed to serialize response body to JSON");

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

        let name_key = find_key("name", &query_pairs);

        let name = match parse_name_query(name_key.cloned()) {
            Ok(c) => c,
            Err(_) => {
                return handle_bad_request("FailedToParseName");
            }
        };

        if name != username {
            return handle_bad_request("Username Not Found");
        }

        let response_body = json!({
          "names": {
            name: pubkey,
          },
          "relays": {
            pubkey: relays,
          }
        });

        let response_body_string = serde_json::to_string(&response_body)
            .expect("Failed to serialize response body to JSON");

        return handle_ok_request(response_body_string);
    }
    handle_ok_request(default_response_body_string)
}
