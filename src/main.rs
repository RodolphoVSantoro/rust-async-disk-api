#![deny(clippy::implicit_return)]
#![allow(clippy::needless_return)]
#![allow(clippy::manual_let_else)]
#![allow(clippy::single_match)]
#![allow(clippy::single_match_else)]
#![allow(clippy::uninlined_format_args)]

mod bank_statement;
mod db;
mod logging;
mod static_responses;
mod transaction;
mod user;

use tokio::{
    io::{AsyncReadExt, AsyncWriteExt},
    net::{TcpListener, TcpStream},
};

#[tokio::main]
async fn main() {
    let args: Vec<String> = std::env::args().collect();
    let default_port = "9999".to_string();
    let port = args.get(1).unwrap_or(&default_port);
    db::init();
    let listener = TcpListener::bind(format!("127.0.0.1:{port}"))
        .await
        .unwrap();
    logging::log!("Listening for connections on port {port}");

    while let Ok((stream, _)) = listener.accept().await {
        tokio::spawn(async move {
            let conn = handle_connection_async(stream).await;
            if let Err(e) = conn {
                logging::log!("An error occurred while handling connection: {}", e);
            }
        });
    }
}

async fn handle_connection_async(mut stream: TcpStream) -> std::io::Result<()> {
    let buffer = &mut [0; 512];
    let request_size = stream.read(buffer).await?;

    logging::log!("Got request");

    if &buffer[0..3] == b"GET" {
        logging::log!("GET");
        return bank_statement::get(stream, buffer, request_size).await;
    }

    if &buffer[0..4] == b"POST" {
        logging::log!("POST");
        return transaction::post(stream, buffer, request_size).await;
    }

    stream
        .write_all(static_responses::METHOD_NOT_ALLOWED)
        .await?;
    Ok(())
}
