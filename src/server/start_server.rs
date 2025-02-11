use hyper::service::{make_service_fn, service_fn};
use hyper::Server;
use std::net::Ipv4Addr;
use tracing::{info, warn};

use crate::server::constants::EnvVariables;
use crate::server::handle_request::handle_request;

pub async fn start_server() -> Result<(), hyper::Error> {
    let default_host = "127.0.0.1".parse::<Ipv4Addr>().unwrap();
    let default_port = 3000;

    let host = match std::env::var(EnvVariables::HOST) {
        Ok(val) => {
            if val.is_empty() {
                info!(target: "server::start_server", "No host specified, using default host: {}", default_host);
                default_host
            } else {
                let res = val.parse::<Ipv4Addr>();
                match res {
                    Ok(res) => {
                        info!(target: "server::start_server", "Using configured host: {}", res);
                        res
                    }
                    Err(e) => {
                        warn!(target: "server::start_server", "Failed to parse host '{}': {}. Using default host: {}", val, e, default_host);
                        default_host
                    }
                }
            }
        }
        Err(_) => {
            info!(target: "server::start_server", "No HOST environment variable found, using default host: {}", default_host);
            default_host
        }
    };

    let port = match std::env::var(EnvVariables::PORT) {
        Ok(val) => {
            if val.is_empty() {
                info!(target: "server::start_server", "No port specified, using default port: {}", default_port);
                default_port
            } else {
                match val.to_string().parse::<u16>() {
                    Ok(port) => {
                        info!(target: "server::start_server", "Using configured port: {}", port);
                        port
                    }
                    Err(e) => {
                        warn!(target: "server::start_server", "Failed to parse port '{}': {}. Using default port: {}", val, e, default_port);
                        default_port
                    }
                }
            }
        }
        Err(_) => {
            info!(target: "server::start_server", "No PORT environment variable found, using default port: {}", default_port);
            default_port
        }
    };

    let addr = (host, port).into();

    let make_svc =
        make_service_fn(|_conn| async { Ok::<_, hyper::Error>(service_fn(handle_request)) });

    let server = Server::bind(&addr).serve(make_svc);
    info!(target: "server::start_server", "Server listening on http://{}", addr);

    server.await.map_err(|e| {
        warn!(target: "server::start_server", "Server error: {}", e);
        e
    })?;

    Ok(())
}
