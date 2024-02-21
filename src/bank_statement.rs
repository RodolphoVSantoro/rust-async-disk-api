use std::sync::Arc;

use crate::{db, responses::ResponseType, transaction::Transaction, user::User};
use serde::{Deserialize, Serialize};
use sqlx::{Pool, Postgres};

#[derive(Serialize, Deserialize, Debug)]
struct StatementResponseSaldo {
    total: i32,
    data_extrato: String,
    limite: i32,
}

#[derive(Serialize, Debug)]
pub struct StatementResponse<'a> {
    saldo: StatementResponseSaldo,
    #[serde(serialize_with = "serialize_transactions")]
    ultimas_transacoes: [Option<&'a Transaction>; 10],
}

pub async fn get(
    pool: Arc<Pool<Postgres>>,
    request: &mut [u8; 512],
    request_size: usize,
) -> ResponseType {
    if request_size < 15 {
        return ResponseType::UnprocessableEntity;
    }
    let id = match get_id(request) {
        Some(id) => id,
        None => {
            return ResponseType::NotFound;
        }
    };

    let mut user = User {
        id,
        balance_limit: 0,
        balance: 0,
        transactions_count: 0,
        last_transaction: 0,
        transactions: Default::default(),
    };
    match db::read_user(pool, id, &mut user).await {
        db::ReadUserResult::Ok => {}
        db::ReadUserResult::NotFound => {
            return ResponseType::NotFound;
        }
        db::ReadUserResult::InternalError(e) => {
            return ResponseType::InternalServerError(e);
        }
    };

    let current_datetime = chrono::Local::now();
    let formatted_datetime = current_datetime.format("%Y-%m-%d %H:%M:%S").to_string();
    let mut ordered_transactions: [Option<&Transaction>; 10] = [None; 10];
    user.get_ordered_transactions(&mut ordered_transactions);

    let statement_response = StatementResponse {
        saldo: StatementResponseSaldo {
            total: user.balance,
            data_extrato: formatted_datetime,
            limite: user.balance_limit,
        },
        ultimas_transacoes: ordered_transactions,
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

fn get_id(request: &[u8]) -> Option<i32> {
    let first_separator = request[13];
    let maybe_id = request[14];
    let second_separator = request[15];
    if first_separator != b'/' || second_separator != b'/' || !maybe_id.is_ascii_digit() {
        return None;
    }
    let id = i32::from(maybe_id);
    let zero_ascii = i32::from(b'0');
    return Some(id - zero_ascii);
}

fn serialize_transactions<S>(v: &[Option<&Transaction>; 10], s: S) -> Result<S::Ok, S::Error>
where
    S: serde::Serializer,
{
    let i = v.iter().position(|x| return x.is_none()).unwrap_or(v.len());
    return v[0..i].serialize(s);
}
