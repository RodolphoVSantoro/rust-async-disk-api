#![deny(clippy::implicit_return)]
#![allow(clippy::needless_return)]
#![allow(clippy::manual_let_else)]
#![allow(clippy::single_match)]
#![allow(clippy::single_match_else)]
#![allow(clippy::uninlined_format_args)]

mod bank_statement;
mod db;
mod logging;
mod responses;
mod transaction;
mod user;

use responses::ResponseType;
use tokio::{io::AsyncReadExt, net::TcpListener};

#[tokio::main]
async fn main() {
    dotenvy::dotenv().ok();
    let args: Vec<String> = std::env::args().collect();
    let default_port = "9999".to_string();
    let port = args.get(1).unwrap_or(&default_port);
    db::init();
    let listener = TcpListener::bind(format!("127.0.0.1:{port}"))
        .await
        .expect(format!("Failed to bind to address {port}").as_str());
    logging::log!("Listening for connections on port {port}");

    while let Ok((mut stream, _)) = listener.accept().await {
        tokio::spawn(async move {
            let request_buffer = &mut [0; 512];
            let read_result = stream.read(request_buffer).await;
            let request_size = match read_result {
                Ok(request_size) => request_size,
                Err(error) => {
                    logging::log!("Failed to read from connection: {}", error);
                    return;
                }
            };

            let response = handle_request(request_buffer, request_size);
            match responses::respond(stream, response).await {
                Ok(()) => {}
                Err(e) => {
                    logging::log!("Failed to write to connection: {}", e);
                }
            };
        });
    }
}

fn handle_request(request: &mut [u8; 512], request_size: usize) -> ResponseType {
    logging::log!("Got request");

    if &request[0..3] == b"GET" {
        logging::log!("GET");
        return bank_statement::get(request, request_size);
    }

    if &request[0..4] == b"POST" {
        logging::log!("POST");
        return transaction::post(request, request_size);
    }

    return ResponseType::MethodNotAllowed;
}
