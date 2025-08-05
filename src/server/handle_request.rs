use crate::server::{constants::CONSTANTS, utils::bech32_encode};
use http::uri::Uri;
use hyper::{http, Body, Request, Response};
use serde::{Deserialize, Serialize};
use serde_json::json;
use tracing::{debug, error, info, warn};

use super::{
    parsing_functions::{
        convert_key, find_key, get_digest, handle_bad_request, handle_ok_request,
        handle_response_body, parse_amount_query, parse_comment_query, parse_name_query,
        parse_nostr_query,
    },
    utils::{create_invoice, get_identifiers},
};
use crate::config::get_config;

pub async fn handle_request(req: Request<Body>) -> Result<Response<Body>, hyper::Error> {
    let method = req.method();
    let path = req.uri().path();

    match (method, path) {
        (&hyper::Method::GET, "/") => {
            debug!(target: "server::handle_request", "Handling default path request");
            handle_default_path()
        }

        (&hyper::Method::GET, "/health") => {
            debug!(target: "server::handle_request", "Handling health check request");
            handle_health_path()
        }

        (&hyper::Method::GET, path) if path.starts_with("/.well-known/lnurlp/") => {
            debug!(target: "server::handle_request", "Handling LNURL payment request for path: {}", path);
            handle_invoice_path(path, req.uri()).await
        }

        (&hyper::Method::GET, path) if path.starts_with("/.well-known/nostr.json") => {
            debug!(target: "server::handle_request", "Handling NIP-05 verification request");
            handle_nip05_path(req.uri()).await
        }
        // Return 404 Not Found for any other requests
        _ => {
            warn!(target: "server::handle_request", "Unknown path requested: {}", path);
            handle_unknown_path()
        }
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

fn handle_health_path() -> Result<Response<Body>, hyper::Error> {
    debug!(target: "server::handle_request::health", "Health check requested");
    let response_body = json!({
      "status": "OK"
    });

    let response_body_string = match serde_json::to_string(&response_body) {
        Ok(body) => body,
        Err(e) => {
            error!(target: "server::handle_request::health", "Failed to serialize health response: {}", e);
            return handle_bad_request("Internal Server Error");
        }
    };

    handle_ok_request(response_body_string)
}

fn handle_default_path() -> Result<Response<Body>, hyper::Error> {
    let (domain, username) = get_identifiers(None);
    debug!(target: "server::handle_request::default", "Using domain: {}, username: {}", domain, username);

    let lnurl_url = "https://".to_owned() + &domain + "/.well-known/lnurlp/" + &username;
    let encoded = bech32_encode("lnurl".to_string(), lnurl_url.clone());

    match encoded {
        Ok(s) => {
            debug!(target: "server::handle_request::default", "Successfully encoded LNURL: {}", s);
            let response_body = DefaultPathResponse {
                lnurl: s,
                decoded_url: lnurl_url.clone(),
                info: Info {
                    title: "RustDress: Lightning Address Personal Server".to_string(),
                    source: "https://github.com/niteshbalusu11/rustdress".to_string(),
                },
            };

            match serde_json::to_string(&response_body) {
                Ok(response_body_string) => handle_ok_request(response_body_string),
                Err(e) => {
                    error!(target: "server::handle_request::default", "Failed to serialize response: {}", e);
                    handle_bad_request("Internal Server Error")
                }
            }
        }
        Err(e) => {
            error!(target: "server::handle_request::default", "Failed to encode LNURL: {:?}", e);
            handle_bad_request("Failed To Encode Lnurl")
        }
    }
}

#[derive(Serialize, Deserialize)]
struct UnknownPathResponse {
    status: String,
    reason: String,
}

fn handle_unknown_path() -> Result<Response<Body>, hyper::Error> {
    warn!(target: "server::handle_request::unknown", "Handling unknown path request");
    let response_body = UnknownPathResponse {
        status: "ERROR".to_string(),
        reason: "Invalid Path".to_string(),
    };

    match serde_json::to_string(&response_body) {
        Ok(response_body_string) => handle_ok_request(response_body_string),
        Err(e) => {
            error!(target: "server::handle_request::unknown", "Failed to serialize error response: {}", e);
            handle_bad_request("Internal Server Error")
        }
    }
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
    info!(target: "server::handle_request::invoice", "Processing invoice request for path: {}", path);
    let username = path.rsplit('/').next();
    let response_body_string = handle_response_body(username);

    info!(target: "server::handle_request::invoice", "Checking username: {:?}", username);

    match username {
        Some(name) if !name.is_empty() => {
            info!(target: "server::handle_request::invoice", "Processing request for username: {}", name);
            if let Some(query_str) = uri.query() {
                debug!(target: "server::handle_request::invoice", "Processing query parameters: {}", query_str);
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
                debug!(target: "server::handle_request::invoice", "Parsed nostr query: {:?}", parsed_nostr_query);

                let digest = get_digest(parsed_nostr_query.as_ref().ok(), Some(name));

                let amount = match parse_amount_query(amount_key.cloned()) {
                    Ok(a) => a,
                    Err(e) => {
                        error!(target: "server::handle_request::invoice", "Failed to parse amount: {:?}", e);
                        return handle_bad_request("UnableToParseAmount");
                    }
                };

                let comment = match parse_comment_query(comment_key.cloned()) {
                    Ok(c) => c,
                    Err(e) => {
                        error!(target: "server::handle_request::invoice", "Failed to parse comment: {:?}", e);
                        return handle_bad_request("FailedToParseComments");
                    }
                };

                if amount == 0 {
                    debug!(target: "server::handle_request::invoice", "Amount is 0, returning payment details");
                    return handle_ok_request(response_body_string);
                }

                debug!(target: "server::handle_request::invoice", "Creating invoice for amount: {}, comment: {}", amount, comment);
                let pr = create_invoice(digest, comment, amount, parsed_nostr_query).await;
                debug!(target: "server::handle_request::invoice", "Created payment request: {}", pr);

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

                match serde_json::to_string(&success_response_body) {
                    Ok(success_response_body_string) => {
                        debug!(target: "server::handle_request::invoice", "Successfully created invoice response");
                        handle_ok_request(success_response_body_string)
                    }
                    Err(e) => {
                        error!(target: "server::handle_request::invoice", "Failed to serialize invoice response: {}", e);
                        handle_bad_request("Internal Server Error")
                    }
                }
            } else {
                debug!(target: "server::handle_request::invoice", "No query parameters, returning payment details");
                handle_ok_request(response_body_string)
            }
        }
        _ => {
            warn!(target: "server::handle_request::invoice", "Username not found in path");
            handle_bad_request("Username Not Found")
        }
    }
}

async fn handle_nip05_path(uri: &Uri) -> Result<Response<Body>, hyper::Error> {
    info!(target: "server::handle_request::nip05", "Processing NIP-05 verification request");

    let config = get_config();

    let relays = CONSTANTS.relays;

    if let Some(query_str) = uri.query() {
        debug!(target: "server::handle_request::nip05", "Processing query parameters: {}", query_str);
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
            Err(e) => {
                error!(target: "server::handle_request::nip05", "Failed to parse name: {:?}", e);
                return handle_bad_request("FailedToParseName");
            }
        };

        let user = config.users.iter().find(|u| u.username == name);

        if let Some(user) = user {
            let pubkey = convert_key(&user.pubkey);
            let response_body = json!({
              "names": {
                &name: &pubkey,
              },
              "relays": {
                &pubkey: &relays,
              }
            });

            match serde_json::to_string(&response_body) {
                Ok(response_body_string) => {
                    info!(target: "server::handle_request::nip05", "Successfully created NIP-05 response for name query");
                    handle_ok_request(response_body_string)
                }
                Err(e) => {
                    error!(target: "server::handle_request::nip05", "Failed to serialize NIP-05 response: {}", e);
                    handle_bad_request("Internal Server Error")
                }
            }
        } else {
            warn!(target: "server::handle_request::nip05", "Username not found: {}", name);
            handle_bad_request("Username Not Found")
        }
    } else {
        let mut names = std::collections::HashMap::new();
        let mut relay_map = std::collections::HashMap::new();

        for user in &config.users {
            let pubkey = convert_key(&user.pubkey);
            names.insert(user.username.clone(), pubkey.clone());
            relay_map.insert(pubkey, relays.to_vec());
        }

        let response_body = json!({
            "names": names,
            "relays": relay_map,
        });

        match serde_json::to_string(&response_body) {
            Ok(default_response_body_string) => {
                info!(target: "server::handle_request::nip05", "Successfully created default NIP-05 response");
                handle_ok_request(default_response_body_string)
            }
            Err(e) => {
                error!(target: "server::handle_request::nip05", "Failed to serialize default NIP-05 response: {}", e);
                handle_bad_request("Internal Server Error")
            }
        }
    }
}
