use serde::{Deserialize, Serialize};
use tokio::{io::AsyncWriteExt, net::TcpStream};

use crate::log;
use crate::{
    db::update_user_with_transaction,
    logging,
    static_responses::{INTERNAL_SERVER_ERROR, NOT_FOUND, UNPROCESSABLE_ENTITY},
};

#[derive(Serialize, Deserialize, Debug)]
pub struct TransactionRequest {
    pub valor: u32,
    pub descricao: String,
    pub tipo: String,
}

#[derive(Serialize, Deserialize, Debug)]
pub struct Transaction {
    pub valor: u32,
    pub descricao: String,
    pub tipo: String,
    pub realizada_em: String,
}

#[derive(Serialize, Deserialize, Debug)]
pub struct PostTransactionResponse {
    pub limite: u32,
    pub saldo: i32,
}

impl Default for Transaction {
    fn default() -> Self {
        Transaction {
            valor: 0,
            descricao: String::new(),
            tipo: String::new(),
            realizada_em: String::new(),
        }
    }
}

// POST /clientes/1/transaction -> 16 bytes
// {"valor": 1, "descricao": "t", "tipo": "c"} -> 38 bytes
// 16 + 38 = 54 for the minimum size of a valid request
const MINIMUM_POST_REQUEST_SIZE: usize = 54;

pub async fn post_transaction(
    mut stream: TcpStream,
    buffer: &mut [u8; 512],
    request_size: usize,
) -> std::io::Result<()> {
    log!("Request size: {}", request_size);
    log!(
        "Buffer: {}",
        logging::into_log_string(&buffer[..request_size])
    );
    if request_size < MINIMUM_POST_REQUEST_SIZE {
        log!("Request too small");
        stream.write(UNPROCESSABLE_ENTITY).await?;
        return Ok(());
    }

    let id = match get_id(buffer) {
        Some(id) => id,
        None => {
            log!("Id not found in request");
            stream.write(NOT_FOUND).await?;
            return Ok(());
        }
    };

    let transaction = match get_body(buffer) {
        Some(transaction) => transaction,
        None => {
            stream.write(NOT_FOUND).await?;
            return Ok(());
        }
    };

    let current_datetime = chrono::Local::now();
    let formatted_datetime = current_datetime.format("%Y-%m-%d %H:%M:%S").to_string();

    let transaction = Transaction {
        valor: transaction.valor,
        descricao: transaction.descricao,
        tipo: transaction.tipo,
        realizada_em: formatted_datetime,
    };

    let user = match update_user_with_transaction(id, &transaction) {
        Ok(Some(user)) => user,
        Ok(None) => {
            stream.write(UNPROCESSABLE_ENTITY).await?;
            return Ok(());
        }
        Err(e) => {
            log!("Error updating user with transaction: {}", e);
            stream.write(INTERNAL_SERVER_ERROR).await?;
            return Ok(());
        }
    };

    let response = PostTransactionResponse {
        limite: user.limit,
        saldo: user.total,
    };

    log!(
        "User {} made a transaction of {} with description {} and type {}",
        id,
        transaction.valor,
        transaction.descricao,
        transaction.tipo
    );

    let response_str = match serde_json::to_string(&response) {
        Ok(response_str) => response_str,
        Err(e) => {
            log!("Error serializing response: {}", e);
            stream.write(INTERNAL_SERVER_ERROR).await?;
            return Ok(());
        }
    };

    let response = format!(
        "HTTP/1.1 200 OK\nContent-Type: application/json\n\n{}",
        response_str
    );
    stream.write(response.as_bytes()).await?;
    Ok(())
}

fn get_id(buffer: &[u8]) -> Option<u32> {
    let first_separator = buffer[14];
    let maybe_id = buffer[15];
    let second_separator = buffer[16];
    if first_separator != b'/' || second_separator != b'/' || maybe_id < b'0' || maybe_id > b'9' {
        return None;
    }
    return Some(maybe_id as u32 - b'0' as u32);
}

fn get_body(buffer: &[u8]) -> Option<TransactionRequest> {
    let body_start = match buffer.iter().position(|&x| x == b'{') {
        Some(index) => index,
        None => {
            log!("Failed to find start of json");
            return None;
        }
    };
    log!("Body start: {}", body_start);
    let mut body_end = match buffer.iter().position(|&x| x == b'}') {
        Some(index) => index,
        None => {
            log!("Failed to find end of json");
            return None;
        }
    };
    body_end += 1;
    log!("Body end: {}", body_end);

    let transaction =
        match serde_json::from_slice::<TransactionRequest>(&buffer[body_start..body_end]) {
            Ok(transaction) => transaction,
            Err(_) => {
                log!(
                    "Failed to parse body from request {}",
                    logging::into_log_string(&buffer[body_start..body_end])
                );
                return None;
            }
        };

    return Some(transaction);
}
