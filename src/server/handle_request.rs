use crate::server::utils::bech32_encode;
use http::uri::Uri;
use hyper::{http, Body, Request, Response};
use serde_json::json;

use super::{
    parsing_functions::{
        find_key, get_digest, handle_bad_request, handle_ok_request, handle_response_body,
        parse_amount_query, parse_comment_query, parse_name_query, parse_nostr_query,
    },
    utils::{create_invoice, get_identifiers, get_nostr_keys},
};

pub async fn handle_request(req: Request<Body>) -> Result<Response<Body>, hyper::Error> {
    match (req.method(), req.uri().path()) {
        (&hyper::Method::GET, "/") => {
            return handle_default_path();
        }

        (&hyper::Method::GET, path) if path.starts_with("/.well-known/lnurlp/") => {
            return handle_invoice_path(path, req.uri()).await
        }

        (&hyper::Method::GET, path) if path.starts_with("/.well-known/nostr.json") => {
            return handle_nip05_path(req.uri()).await
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
    let username = path.rsplitn(2, '/').next();
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

async fn handle_nip05_path(uri: &Uri) -> Result<Response<Body>, hyper::Error> {
    println!("inside nip05 {:?}", uri);
    let pubkey = match get_nostr_keys() {
        Ok((_, key)) => key,
        Err(_) => return handle_bad_request("Failed To Get Nostr Keys"),
    };

    let relays = vec![
        "wss://nostr.foundrydigital.com",
        "wss://eden.nostr.land",
        "wss://relay.damus.io",
        "wss://relay.snort.social",
        "wss://nos.lol",
        "wss://relay.current.fyi",
        "wss://relay.nostr.info",
        "wss://nostr.zebedee.cloud",
        "wss://nostr.fmt.wiz.biz",
        "wss://relay.nostr.bg",
        "wss://nostr.mom",
        "wss://nostr.bitcoiner.social",
        "wss://nostr.oxtr.dev",
        "wss://no.str.cr",
    ];

    let relay_string: Vec<String> = relays.iter().map(|s| s.to_string()).collect();

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
                return handle_bad_request("FailedToParseComments");
            }
        };

        let username = std::env::var("USERNAME").unwrap();

        if name != username {
            return handle_bad_request("Username Not Found");
        }

        let response_body = json!({
          "names": {
            name: pubkey,
          },
          "relays": {
            pubkey: relay_string,
          }
        });

        let response_body_string = serde_json::to_string(&response_body)
            .expect("Failed to serialize response body to JSON");

        println!("{:?}", response_body_string);

        return handle_ok_request(response_body_string);
    } else {
        return handle_bad_request("Query Not Found");
    }
}
