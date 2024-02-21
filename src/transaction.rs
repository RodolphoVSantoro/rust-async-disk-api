use std::sync::Arc;

use serde::{Deserialize, Serialize};
use sqlx::{Pool, Postgres};

use crate::{
    db::{update_user_with_transaction, UpdateUserResult},
    logging,
    responses::ResponseType,
};

#[derive(Serialize, Deserialize, Debug)]
struct TransactionRequest {
    valor: i32,
    descricao: String,
    tipo: String,
}

#[derive(Serialize, Deserialize, Debug)]
pub struct Transaction {
    pub valor: i32,
    pub descricao: String,
    pub tipo: String,
    pub realizada_em: String,
}

#[derive(Serialize, Deserialize, Debug)]
struct PostTransactionResponse {
    limite: i32,
    saldo: i32,
}

impl Default for Transaction {
    fn default() -> Self {
        return Transaction {
            valor: 0,
            descricao: String::new(),
            tipo: String::new(),
            realizada_em: String::new(),
        };
    }
}

// POST /clientes/1/transaction -> 16 bytes
// {"valor": 1, "descricao": "t", "tipo": "c"} -> 38 bytes
// 16 + 38 = 54 for the minimum size of a valid request
const MINIMUM_POST_REQUEST_SIZE: usize = 54;

pub async fn post(
    pool: Arc<Pool<Postgres>>,
    request: &mut [u8; 512],
    request_size: usize,
) -> ResponseType {
    logging::log!("Request size: {}", request_size);
    logging::log!(
        "Request: {}",
        logging::into_log_json(&request[..request_size])
    );
    if request_size < MINIMUM_POST_REQUEST_SIZE {
        logging::error!("Request too small: {}", String::from_utf8_lossy(request));
        return ResponseType::UnprocessableEntity;
    }

    let id = match get_id(request) {
        Some(id) => id,
        None => {
            logging::log!("Id not found in request");
            return ResponseType::UnprocessableEntity;
        }
    };

    let transaction = match get_body(request) {
        Some(transaction) => transaction,
        None => {
            return ResponseType::UnprocessableEntity;
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

    let user = match update_user_with_transaction(pool, id, &transaction).await {
        UpdateUserResult::Ok(user) => user,
        UpdateUserResult::Unprocessable(err) => {
            logging::log!("Unprocessable entity: {}", err);
            return ResponseType::UnprocessableEntity;
        }
        UpdateUserResult::NotFound => {
            logging::log!("User {} not found on update", id);
            return ResponseType::NotFound;
        }
        UpdateUserResult::InternalError(error) => {
            return ResponseType::InternalServerError(error);
        }
    };

    let response = PostTransactionResponse {
        limite: user.balance_limit,
        saldo: user.balance,
    };

    logging::log!(
        "User {} made a transaction of {} with description {} and type {}",
        id,
        transaction.valor,
        transaction.descricao,
        transaction.tipo
    );

    let response_str = match serde_json::to_string(&response) {
        Ok(response_str) => response_str,
        Err(e) => {
            let error_string = format!("Error serializing response: {}", e);
            return ResponseType::InternalServerError(error_string);
        }
    };

    return ResponseType::Ok(response_str);
}

fn get_id(request: &[u8]) -> Option<i32> {
    let first_separator = request[14];
    let maybe_id = request[15];
    let second_separator = request[16];
    if first_separator != b'/' || second_separator != b'/' || !maybe_id.is_ascii_digit() {
        return None;
    }
    let id = i32::from(maybe_id);
    let zero_ascii: i32 = i32::from(b'0');
    return Some(id - zero_ascii);
}

fn get_body(request: &[u8]) -> Option<TransactionRequest> {
    let mut request_iter = request.iter();
    let body_start = match request_iter.position(|&x| return x == b'{') {
        Some(index) => index,
        None => {
            logging::log!("Failed to find start of json");
            return None;
        }
    };
    logging::log!("Body start: {}", body_start);
    let mut body_end = match request_iter.position(|&x| return x == b'}') {
        Some(index) => index,
        None => {
            logging::log!("Failed to find end of json");
            return None;
        }
    };
    body_end = body_end + body_start + 2;
    logging::log!("Body end: {}", body_end);

    let transaction =
        match serde_json::from_slice::<TransactionRequest>(&request[body_start..body_end]) {
            Ok(transaction) => transaction,
            Err(_) => {
                logging::log!(
                    "Failed to parse body from request {}",
                    logging::into_log_json(&request[body_start..body_end])
                );
                return None;
            }
        };

    return Some(transaction);
}

pub fn encode_transactions(transactions: &[Transaction; 10]) -> Vec<u8> {
    let encoded_transactions =
        bincode::serialize(transactions).expect("Failed to encode transactions");
    assert!(
        encoded_transactions.len() < 1000,
        "Encoded transactions too large"
    );
    return encoded_transactions;
}

pub fn decode_transactions(
    encoded_transactions: Option<Vec<u8>>,
    transactions: &mut [Transaction; 10],
) {
    if encoded_transactions.is_none() {
        return;
    }
    let encoded_transactions = encoded_transactions.unwrap();
    *transactions =
        bincode::deserialize(&encoded_transactions).expect("Failed to decode transactions");
}
