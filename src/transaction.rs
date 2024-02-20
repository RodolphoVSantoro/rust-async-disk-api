use serde::{Deserialize, Serialize};

use crate::{
    db::{update_user_with_transaction, UpdateUserResult},
    logging,
    responses::ResponseType,
};

#[derive(Serialize, Deserialize, Debug)]
struct TransactionRequest {
    valor: u32,
    descricao: String,
    tipo: String,
}

#[derive(Serialize, Deserialize, Debug)]
pub struct Transaction {
    pub valor: u32,
    pub descricao: String,
    pub tipo: String,
    pub realizada_em: String,
}

#[derive(Serialize, Deserialize, Debug)]
struct PostTransactionResponse {
    limite: u32,
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

pub fn post(buffer: &mut [u8; 512], request_size: usize) -> ResponseType {
    logging::log!("Request size: {}", request_size);
    logging::log!(
        "Buffer: {}",
        logging::into_log_string(&buffer[..request_size])
    );
    if request_size < MINIMUM_POST_REQUEST_SIZE {
        logging::log!("Request too small");
        return ResponseType::UnprocessableEntity;
    }

    let id = match get_id(buffer) {
        Some(id) => id,
        None => {
            logging::log!("Id not found in request");
            return ResponseType::UnprocessableEntity;
        }
    };

    let transaction = match get_body(buffer) {
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

    let user = match update_user_with_transaction(id, &transaction) {
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
        limite: user.limit,
        saldo: user.total,
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

fn get_id(buffer: &[u8]) -> Option<u32> {
    let first_separator = buffer[14];
    let maybe_id = buffer[15];
    let second_separator = buffer[16];
    if first_separator != b'/' || second_separator != b'/' || !maybe_id.is_ascii_digit() {
        return None;
    }
    let id = u32::from(maybe_id);
    let zero_ascii = u32::from(b'0');
    return Some(id - zero_ascii);
}

fn get_body(buffer: &[u8]) -> Option<TransactionRequest> {
    let body_start = match buffer.iter().position(|&x| return x == b'{') {
        Some(index) => index,
        None => {
            logging::log!("Failed to find start of json");
            return None;
        }
    };
    logging::log!("Body start: {}", body_start);
    let mut body_end = match buffer[body_start..].iter().position(|&x| return x == b'}') {
        Some(index) => index,
        None => {
            logging::log!("Failed to find end of json");
            return None;
        }
    };
    body_end = body_end + body_start + 1;
    logging::log!("Body end: {}", body_end);

    let transaction =
        match serde_json::from_slice::<TransactionRequest>(&buffer[body_start..body_end]) {
            Ok(transaction) => transaction,
            Err(_) => {
                logging::log!(
                    "Failed to parse body from request {}",
                    logging::into_log_string(&buffer[body_start..body_end])
                );
                return None;
            }
        };

    return Some(transaction);
}
