// use lazy_static::lazy_static;

pub fn into_log_string(buffer: &[u8]) -> String {
    return String::from_utf8_lossy(buffer)
        .replace("\n", "\\n")
        .replace("\r", "\\r")
        .replace("\t", "\\t")
        .replace("\0", "\\0");
}

//TODO: get from env
// lazy_static! {
//     pub static ref ENABLE_LOG: bool = match std::env::var("ENABLE_LOG") {
//         Ok(val) => val == "true" || val == "1",
//         Err(_) => false,
//     };
// }
pub const ENABLE_LOG: bool = false;

#[macro_export]
macro_rules! log {
    ($($arg:tt)*) => {
        if crate::logging::ENABLE_LOG {
            println!($($arg)*);
        }
    };
}
