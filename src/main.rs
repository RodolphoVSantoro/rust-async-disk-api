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

use std::sync::Arc;

use responses::ResponseType;
use sqlx::{postgres::PgPoolOptions, Pool, Postgres};
use tokio::{io::AsyncReadExt, net::TcpSocket};

#[tokio::main(flavor = "current_thread")]
async fn main() {
    dotenvy::dotenv().ok();
    let args: Vec<String> = std::env::args().collect();
    let default_port = "9999".to_string();
    let port = args.get(1).unwrap_or(&default_port);
    let should_init_db = args.get(2).map(|s| return s.as_str()) == Some("resetDb");
    let max_connections = std::env::var("MAX_CONNECTIONS")
        .ok()
        .and_then(|m| return m.parse::<u32>().ok())
        .unwrap_or(15);
    println!("Max database connections: {max_connections}");

    let pool_result = PgPoolOptions::new()
        .max_connections(max_connections)
        .connect(&std::env::var("DATABASE_URL").unwrap())
        .await;

    let pool = match pool_result {
        Ok(pool) => Arc::new(pool),
        Err(e) => {
            panic!("Failed to connect to database: {}", e);
        }
    };
    if should_init_db {
        logging::log!("Resetting db");
        db::reset(&pool).await;
    }
    let addr: std::net::SocketAddr = format!("127.0.0.1:{port}").parse().unwrap();
    let socket = TcpSocket::new_v4().expect("Failed to create socket");
    socket
        .set_reuseaddr(true)
        .expect("Failed to set reuse address");
    socket.bind(addr).expect("Failed to bind to address");

    let listener = socket.listen(2048).expect("Failed to listen on address");

    logging::log!("Listening for connections on port {port}");

    while let Ok((mut stream, _)) = listener.accept().await {
        let pool_clone = pool.clone();
        let request_buffer = &mut [0; 512];
        let read_result = stream.read(request_buffer).await;
        let request_size = match read_result {
            Ok(request_size) => request_size,
            Err(error) => {
                logging::error!("Failed to read from connection: {}", error);
                return;
            }
        };

        let response = handle_request(pool_clone, request_buffer, request_size).await;
        match responses::respond(stream, response).await {
            Ok(()) => {}
            Err(e) => {
                logging::error!("Failed to write to connection: {}", e);
            }
        };
    }
}

async fn handle_request(
    pool: Arc<Pool<Postgres>>,
    request: &mut [u8; 512],
    request_size: usize,
) -> ResponseType {
    logging::log!("Got request");

    if &request[0..3] == b"GET" {
        logging::log!("GET");
        return bank_statement::get(pool, request, request_size).await;
    }

    if &request[0..4] == b"POST" {
        logging::log!("POST");
        return transaction::post(pool, request, request_size).await;
    }

    return ResponseType::MethodNotAllowed;
}
