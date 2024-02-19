use std::sync::OnceLock;

pub fn into_log_string(buffer: &[u8]) -> String {
    return String::from_utf8_lossy(buffer)
        .replace('\n', "\\n")
        .replace('\r', "\\r")
        .replace('\t', "\\t")
        .replace('\0', "\\0");
}

pub fn get_enable_log() -> &'static bool {
    static INIT: OnceLock<bool> = OnceLock::new();
    return INIT.get_or_init(|| {
        return match std::env::var("ENABLE_LOG") {
            Ok(val) => val == "true" || val == "1",
            Err(_) => false,
        };
    });
}

// pub const ENABLE_LOG: bool = false;

macro_rules! log {
    ($($arg:expr),*) => {
        if *crate::logging::get_enable_log() {
            println!($($arg),*);
        }
    };
}

pub(crate) use log;
