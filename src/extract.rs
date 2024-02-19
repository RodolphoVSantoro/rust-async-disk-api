use crate::{
    db, log,
    static_responses::{INTERNAL_SERVER_ERROR, NOT_FOUND, UNPROCESSABLE_ENTITY},
    transaction::Transaction,
};
use serde::{Deserialize, Serialize};
use tokio::{io::AsyncWriteExt, net::TcpStream};

#[derive(Serialize, Deserialize, Debug)]
struct ExtractResponseSaldo {
    total: i32,
    data_extrato: String,
    limite: u32,
}

pub struct ExtractResponse<'a> {
    saldo: ExtractResponseSaldo,
    ultimas_transacoes: Vec<&'a Transaction>,
}

pub async fn get_extract(
    mut stream: TcpStream,
    buffer: &mut [u8; 512],
    request_size: usize,
) -> std::io::Result<()> {
    if request_size < 15 {
        stream.write(UNPROCESSABLE_ENTITY).await?;
        return Ok(());
    }
    let id = match get_id(buffer) {
        Some(id) => id,
        None => {
            stream.write(NOT_FOUND).await?;
            return Ok(());
        }
    };

    let user = match db::read_user(id) {
        Ok(user) => user,
        Err(e) => {
            log!("Error reading user: {}", e);
            stream.write(INTERNAL_SERVER_ERROR).await?;
            return Ok(());
        }
    };

    let current_datetime = chrono::Local::now();
    let formatted_datetime = current_datetime.format("%Y-%m-%d %H:%M:%S").to_string();

    let extract_response = ExtractResponse {
        saldo: ExtractResponseSaldo {
            total: user.total,
            data_extrato: formatted_datetime,
            limite: user.limit,
        },
        ultimas_transacoes: user.get_ordered_transactions(),
    };

    let response_body = serialize_extract_response(extract_response);

    let response = format!("HTTP/1.1 200 OK\nContent-Type: application/json\n\n{response_body}",);
    stream.write(response.as_bytes()).await?;
    Ok(())
}

pub fn get_id(buffer: &[u8]) -> Option<u32> {
    let first_separator = buffer[13];
    let maybe_id = buffer[14];
    let second_separator = buffer[15];
    if first_separator != b'/' || second_separator != b'/' || maybe_id < b'0' || maybe_id > b'9' {
        return None;
    }
    return Some(maybe_id as u32 - b'0' as u32);
}

pub fn serialize_extract_response(extract_response: ExtractResponse) -> String {
    let mut response_body = String::from("{");

    response_body.push_str(&format!(
        "\"saldo\": {},",
        serde_json::to_string(&extract_response.saldo).unwrap()
    ));
    response_body.push_str(&format!("\"ultimas_transacoes\": ["));

    for transaction in extract_response.ultimas_transacoes {
        response_body.push_str(&serde_json::to_string(transaction).unwrap());
        response_body.push_str(",");
    }
    response_body.push_str("]}");

    return response_body;
}
