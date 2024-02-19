mod db;
mod extract;
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
    db::init_db();
    let listener = TcpListener::bind("127.0.0.1:9999").await.unwrap();
    log!("Listening for connections on port {}", 9999);

    while let Ok((stream, _)) = listener.accept().await {
        tokio::spawn(async move {
            let conn = handle_connection_async(stream).await;
            if let Err(e) = conn {
                log!("An error occurred while handling connection: {}", e);
            }
        });
    }
}

async fn handle_connection_async(mut stream: TcpStream) -> std::io::Result<()> {
    let buffer = &mut [0; 512];
    let request_size = stream.read(buffer).await?;

    log!("Got request");

    if &buffer[0..3] == b"GET" {
        log!("GET");
        return extract::get_extract(stream, buffer, request_size).await;
    }

    if &buffer[0..4] == b"POST" {
        log!("POST");
        return transaction::post_transaction(stream, buffer, request_size).await;
    }

    stream.write(static_responses::METHOD_NOT_ALLOWED).await?;
    Ok(())
}
