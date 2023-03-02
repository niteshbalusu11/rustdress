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
            let response_body = json!({ "lnurl": s, "decoded_url": lnurl_url.clone(), "info": {"title": "RustDress: Lightning Address Personal Server", "source": "TODO"}  });
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
        ["text/plain", &("Satoshis to ".to_owned() + &identifier)],
    ])
    .expect("Failed to serialize metadata");

    let digest = digest::digest(&digest::SHA256, metadata.as_bytes())
        .as_ref()
        .to_vec();

    let lnurl_url = "https://".to_owned() + &domain + "/.well-known/lnurlp/" + username.as_str();

    let response_body = json!({ "status": "OK", "callback": lnurl_url, "tag": "payRequest", "maxSendable": 100000000, "minSendable": 1000, "commentAllowed": 0, "metadata": metadata});
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

                let key_found = find_key("amount", &query_pairs);

                match key_found {
                    Some(k) => {
                        let (_, a) = k;
                        let amount = a.parse::<i64>();

                        match amount {
                            Ok(a) => {
                                if a < 1000 {
                                    return handle_bad_request(
                                        "Expted Amount Greater Than 1000mSat",
                                    );
                                }

                                if a > 100000000 {
                                    return handle_bad_request(
                                        "Expted Amount Greater Than 1000mSat",
                                    );
                                }
                            }

                            Err(_) => return handle_bad_request("Unable To Parse Amount"),
                        };

                        let mut lnd = get_lnd().await;

                        let result = lnd
                            .lightning()
                            .add_invoice(Invoice {
                                value_msat: amount.unwrap(),
                                expiry: 300,
                                private: add_hop_hints(),
                                description_hash: digest,
                                ..Default::default()
                            })
                            .await
                            .expect("FailedToAuthenticateToLnd");
                        let pr = result.into_inner().payment_request;

                        let response_body = json!({
                            "status": "OK",
                            "routes": [],
                            "successAction": { "tag": "message", "message": "Payment received!" },
                            "pr": pr,
                            "disposable": false,
                        });

                        let response_body_string = serde_json::to_string(&response_body)
                            .expect("Failed to serialize response body to JSON");

                        return handle_ok_request(response_body_string);
                    }
                    _ => return handle_bad_request("Invalid Query Parameters"),
                }
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
        .body(Body::from(response_body_string))
        .unwrap();
    Ok(resp)
}

fn handle_ok_request(body: String) -> Result<Response<Body>, hyper::Error> {
    let resp = Response::builder()
        .status(StatusCode::OK)
        .header("content-type", "application/json")
        .body(Body::from(body))
        .unwrap();
    Ok(resp)
}
