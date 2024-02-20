use tokio::{
    io::AsyncWriteExt,
    net::TcpStream,
};
use crate::logging;

pub const METHOD_NOT_ALLOWED: &[u8] = 
    b"HTTP/1.1 405 Method Not Allowed\nContent-Type: application/json\n\n{\"message\": \"Method Not Allowed\"}";

#[allow(dead_code)]
pub const BAD_REQUEST: &[u8] =
    b"HTTP/1.1 400 Bad Request\nContent-Type: application/json\n\n{\"message\": \"Bad Request\"}}";

    pub const NOT_FOUND: &[u8] =
    b"HTTP/1.1 404 Not Found\nContent-Type: application/json\n\n{\"message\": \"Not Found\"}";

#[allow(dead_code)]
pub const UNPROCESSABLE_ENTITY: &[u8] =
    b"HTTP/1.1 422 Unprocessable Entity\nContent-Type: application/json\n\n{\"message\": \"Unprocessable Entity\"}";

#[allow(dead_code)]
pub const INTERNAL_SERVER_ERROR: &[u8] =
    b"HTTP/1.1 500 Internal Server Error\nContent-Type: application/json\n\n{\"message\": \"Internal Server Error\"}";

pub enum ResponseType{
    // Ok(String) is the response body
    Ok(String),
    // InternalServerError(String) is the error message to log
    InternalServerError(String),
    NotFound,
    MethodNotAllowed,
    UnprocessableEntity,
}

pub async fn respond(mut stream: TcpStream, response: ResponseType) -> std::io::Result<()> {
    return match response {
        ResponseType::Ok(response_body) => {
            let response = format!("HTTP/1.1 200 OK\nContent-Type: application/json\n\n{response_body}");
            stream.write_all(response.as_bytes()).await
        }
        ResponseType::InternalServerError(error_string) => {
            logging::log!("Internal server error {error_string}");
            stream.write_all(INTERNAL_SERVER_ERROR).await
        }
        ResponseType::NotFound => {
            stream.write_all(NOT_FOUND).await
        }
        ResponseType::MethodNotAllowed => {
            stream.write_all(METHOD_NOT_ALLOWED).await
        }
        ResponseType::UnprocessableEntity => {
            stream.write_all(UNPROCESSABLE_ENTITY).await
        }
    };
}