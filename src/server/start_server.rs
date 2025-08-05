use hyper::service::{make_service_fn, service_fn};
use hyper::Server;
use std::net::Ipv4Addr;
use tracing::{info, warn};

use crate::config::get_config;
use crate::server::handle_request::handle_request;

pub async fn start_server() -> Result<(), hyper::Error> {
    let config = get_config();
    let server_config = config.server.clone();

    let host = server_config.host.parse::<Ipv4Addr>().unwrap_or_else(|e| {
        warn!(target: "server::start_server", "Failed to parse host '{}': {}. Using default host: 127.0.0.1", server_config.host, e);
        "127.0.0.1".parse().unwrap()
    });
    let port = server_config.port;

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
