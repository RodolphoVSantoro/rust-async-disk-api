use crate::{
    db, logging,
    static_responses::{INTERNAL_SERVER_ERROR, NOT_FOUND, UNPROCESSABLE_ENTITY},
    transaction::Transaction,
};
use serde::{Deserialize, Serialize};
use tokio::{io::AsyncWriteExt, net::TcpStream};

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

pub async fn get(
    mut stream: TcpStream,
    buffer: &mut [u8; 512],
    request_size: usize,
) -> std::io::Result<()> {
    if request_size < 15 {
        stream.write_all(UNPROCESSABLE_ENTITY).await?;
        return Ok(());
    }
    let id = match get_id(buffer) {
        Some(id) => id,
        None => {
            stream.write_all(NOT_FOUND).await?;
            return Ok(());
        }
    };

    let user = match db::read_user(id) {
        Ok(user) => user,
        Err(e) => {
            if e.kind() == std::io::ErrorKind::NotFound {
                stream.write_all(NOT_FOUND).await?;
                return Ok(());
            }
            logging::log!("Error reading user: {}", e);
            stream.write_all(INTERNAL_SERVER_ERROR).await?;
            return Ok(());
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

    let response_body = serde_json::to_string(&statement_response).unwrap();

    let response = format!("HTTP/1.1 200 OK\nContent-Type: application/json\n\n{response_body}",);
    stream.write_all(response.as_bytes()).await?;
    Ok(())
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
