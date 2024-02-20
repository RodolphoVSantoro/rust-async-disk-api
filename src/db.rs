use std::fs::File;
use std::io::{BufReader, BufWriter, Read, Seek, Write};

use crate::logging;
use crate::transaction::Transaction;
use crate::user::{TransactionResult, User};

use fs4::FileExt;

const INITIAL_USER_LIMITS: [u32; 5] = [100_000, 80_000, 1_000_000, 10_000_000, 500_000];
pub fn init() {
    match std::fs::create_dir_all("./data") {
        Ok(()) => {
            logging::log!("Data directory created successfully!");
        }
        Err(e) => {
            panic!("Error creating data directory: {}", e);
        }
    };
    logging::log!("Initializing database");
    for (i, limit) in INITIAL_USER_LIMITS.iter().enumerate() {
        let i: u32 = i.try_into().expect("Failed to convert i to u32");
        match create_user(i + 1, *limit) {
            CreateUserResult::Ok => {
                logging::log!("User {} created", i + 1);
            }
            CreateUserResult::InternalError(e) => {
                panic!("Error creating user: {}", e);
            }
        }
    }
    logging::log!("Database initialized successfully!");
}

pub enum ReadUserResult {
    Ok(User),
    NotFound,
    InternalError(String),
}

pub fn read_user(id: u32) -> ReadUserResult {
    let file_path = format!("data/user{}.bin", id);
    let file = match File::open(file_path) {
        Ok(file) => file,
        Err(e) => match e.kind() {
            std::io::ErrorKind::NotFound => {
                return ReadUserResult::NotFound;
            }
            _ => {
                let error_string = format!("Error opening file: {}", e);
                return ReadUserResult::InternalError(error_string);
            }
        },
    };
    match file.lock_shared() {
        Ok(()) => {}
        Err(e) => {
            let error_string = format!("Error locking file for read: {}", e);
            return ReadUserResult::InternalError(error_string);
        }
    };
    let mut buff_reader = BufReader::new(file);

    let mut serialized_user = Vec::new();
    match buff_reader.read_to_end(&mut serialized_user) {
        Ok(_) => {}
        Err(e) => {
            let error_string = format!("Error reading from file: {}", e);
            return ReadUserResult::InternalError(error_string);
        }
    }

    return match bincode::deserialize(&serialized_user) {
        Ok(user) => ReadUserResult::Ok(user),
        Err(e) => {
            let error_string = format!("Error deserializing user: {}", e);
            return ReadUserResult::InternalError(error_string);
        }
    };
}

pub enum CreateUserResult {
    Ok,
    InternalError(String),
}

pub fn create_user(id: u32, limit: u32) -> CreateUserResult {
    let file_path = format!("data/user{}.bin", id);
    let file = match File::create(file_path) {
        Ok(file) => file,
        Err(e) => {
            let error_string = format!("Error opening file: {}", e);
            return CreateUserResult::InternalError(error_string);
        }
    };
    match file.lock_exclusive() {
        Ok(()) => {}
        Err(e) => {
            let error_string = format!("Error locking file for write: {}", e);
            return CreateUserResult::InternalError(error_string);
        }
    };
    let mut buff_writer = BufWriter::new(file);
    let user = User {
        limit,
        total: 0,
        n_transactions: 0,
        last_transaction: 0,
        transactions: Default::default(),
    };
    let serialized_user = match bincode::serialize(&user) {
        Ok(serialized_user) => serialized_user,
        Err(e) => {
            let error_string = format!("Error serializing user: {}", e);
            return CreateUserResult::InternalError(error_string);
        }
    };
    match buff_writer.write_all(&serialized_user) {
        Ok(()) => {}
        Err(e) => {
            let error_string = format!("Error writing to file: {}", e);
            return CreateUserResult::InternalError(error_string);
        }
    }

    return match buff_writer.flush() {
        Ok(()) => CreateUserResult::Ok,
        Err(e) => {
            let error_string = format!("Error flushing to file: {}", e);
            return CreateUserResult::InternalError(error_string);
        }
    };
}

pub enum UpdateUserResult {
    Ok(User),
    NotFound,
    Unprocessable(String),
    InternalError(String),
}

pub fn update_user_with_transaction(id: u32, transaction: &Transaction) -> UpdateUserResult {
    // init file and buffers
    let file_path = format!("data/user{}.bin", id);
    let file_result = std::fs::OpenOptions::new()
        .write(true)
        .read(true)
        .open(file_path);
    let file = match file_result {
        Ok(file) => file,
        Err(e) => match e.kind() {
            std::io::ErrorKind::NotFound => {
                return UpdateUserResult::NotFound;
            }
            _ => {
                let error_string = format!("Error opening file: {}", e);
                return UpdateUserResult::InternalError(error_string);
            }
        },
    };

    let mut buff_reader = BufReader::new(&file);
    let mut buff_writer = BufWriter::new(&file);

    match file.lock_exclusive() {
        Ok(()) => {}
        Err(e) => {
            let error_string = format!("Error locking file for read: {}", e);
            return UpdateUserResult::InternalError(error_string);
        }
    };

    // read serialized user from file
    let mut serialized_user = Vec::new();
    match buff_reader.read_to_end(&mut serialized_user) {
        Ok(_) => {}
        Err(e) => {
            let error_string = format!("Error reading from file: {}", e);
            return UpdateUserResult::InternalError(error_string);
        }
    }

    // deserialize user
    let mut user: User = match bincode::deserialize(&serialized_user) {
        Ok(user) => user,
        Err(e) => {
            let error_string = format!("Error deserializing user: {}", e);
            return UpdateUserResult::InternalError(error_string);
        }
    };

    // compute transaction and update user if it is valid
    match user.compute_transaction(transaction) {
        TransactionResult::Ok => {
            logging::log!("Transaction computed successfully! Adding to list of transactions.");
            user.add_transaction(transaction);
        }
        // Return None if the transaction is invalid
        TransactionResult::LimitExceeded => {
            let error_string = format!("Limit exceeded for user {}", id);
            return UpdateUserResult::Unprocessable(error_string);
        }
        TransactionResult::InvalidDescription => {
            let error_string = format!("Invalid description for user {}", id);
            return UpdateUserResult::Unprocessable(error_string);
        }
        TransactionResult::InvalidTransactionKind(t) => {
            let error_string = format!("Invalid transaction kind {} for user {}", t, id);
            return UpdateUserResult::Unprocessable(error_string);
        }
    };

    // serialize updated user
    let serialized_user = match bincode::serialize(&user) {
        Ok(serialized_user) => serialized_user,
        Err(e) => {
            let error_string = format!("Error serializing user: {}", e);
            return UpdateUserResult::InternalError(error_string);
        }
    };

    // move to start of file
    match buff_reader.seek(std::io::SeekFrom::Start(0)) {
        Ok(_) => {}
        Err(e) => {
            let error_string = format!("Error seeking to start of file: {}", e);
            return UpdateUserResult::InternalError(error_string);
        }
    };
    // write updated user to file
    match buff_writer.write_all(&serialized_user) {
        Ok(()) => {}
        Err(e) => {
            let error_string = format!("Error writing to file: {}", e);
            return UpdateUserResult::InternalError(error_string);
        }
    }

    return UpdateUserResult::Ok(user);
}
