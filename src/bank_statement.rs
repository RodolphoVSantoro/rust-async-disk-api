use crate::{db, responses::ResponseType, transaction::Transaction};
use serde::{Deserialize, Serialize};

#[derive(Serialize, Deserialize, Debug)]
struct StatementResponseSaldo {
    total: i32,
    data_extrato: String,
    limite: u32,
}

#[derive(Serialize, Debug)]
pub struct StatementResponse<'a> {
    saldo: StatementResponseSaldo,
    ultimas_transacoes: Vec<&'a Transaction>,
}

pub fn get(buffer: &mut [u8; 512], request_size: usize) -> ResponseType {
    if request_size < 15 {
        return ResponseType::UnprocessableEntity;
    }
    let id = match get_id(buffer) {
        Some(id) => id,
        None => {
            return ResponseType::NotFound;
        }
    };

    let user = match db::read_user(id) {
        db::ReadUserResult::Ok(user) => user,
        db::ReadUserResult::NotFound => {
            return ResponseType::NotFound;
        }
        db::ReadUserResult::InternalError(e) => {
            return ResponseType::InternalServerError(e);
        }
    };

    let current_datetime = chrono::Local::now();
    let formatted_datetime = current_datetime.format("%Y-%m-%d %H:%M:%S").to_string();

    let statement_response = StatementResponse {
        saldo: StatementResponseSaldo {
            total: user.total,
            data_extrato: formatted_datetime,
            limite: user.limit,
        },
        ultimas_transacoes: user.get_ordered_transactions(),
    };

    let serialize_result = serde_json::to_string(&statement_response);
    return match serialize_result {
        Ok(response_body) => ResponseType::Ok(response_body),
        Err(e) => {
            let error_string = format!("Error serializing response: {}", e);
            return ResponseType::InternalServerError(error_string);
        }
    };
}

fn get_id(buffer: &[u8]) -> Option<u32> {
    let first_separator = buffer[13];
    let maybe_id = buffer[14];
    let second_separator = buffer[15];
    if first_separator != b'/' || second_separator != b'/' || !maybe_id.is_ascii_digit() {
        return None;
    }
    let id = u32::from(maybe_id);
    let zero_ascii = u32::from(b'0');
    return Some(id - zero_ascii);
}
