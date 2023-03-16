use dotenv::dotenv;
use hyper::service::{make_service_fn, service_fn};
use hyper::Server;
use std::net::Ipv4Addr;

use crate::server::constants::EnvVariables;
use crate::server::handle_request::handle_request;

pub async fn start_server() {
    let default_host = "127.0.0.1".parse::<Ipv4Addr>().unwrap();
    let default_port = 3000;

    dotenv().ok();

    let host = match std::env::var(EnvVariables::HOST) {
        Ok(val) => {
            if val.is_empty() {
                default_host
            } else {
                let res = val.parse::<Ipv4Addr>();
                match res {
                    Ok(res) => res,
                    Err(_) => {
                        println!("Failed To Parse Host, Returning Default Host");
                        default_host
                    }
                }
            }
        }
        Err(_) => default_host,
    };

    let port = match std::env::var(EnvVariables::PORT) {
        Ok(val) => {
            if val.is_empty() {
                default_port
            } else {
                let res = val.to_string().parse::<u16>();
                match res {
                    Ok(res) => res,
                    Err(_) => {
                        println!("Failed To Parse Port, Returning Default Port");
                        default_port
                    }
                }
            }
        }
        Err(_) => default_port,
    };

    let addr = (host, port).into();

    let make_svc =
        make_service_fn(|_conn| async { Ok::<_, hyper::Error>(service_fn(handle_request)) });

    let server = Server::bind(&addr).serve(make_svc);
    println!("Listening on http://{}", addr);

    let res = server.await;

    match res {
        Err(e) => panic!("FailedToStartHttpServer {}", e),
        _ => return,
    }
}
