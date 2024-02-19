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
