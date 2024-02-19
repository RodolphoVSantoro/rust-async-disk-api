use std::fs::File;
use std::io::{BufReader, BufWriter, Read, Seek, Write};

use crate::log;
use crate::transaction::Transaction;
use crate::user::{TransactionResult, User};

use fs4::FileExt;

pub fn init_db() {
    const INITIAL_USER_LIMITS: [u32; 5] = [100000, 80000, 1000000, 10000000, 500000];
    log!("Initializing database");
    for (i, limit) in INITIAL_USER_LIMITS.iter().enumerate() {
        log!("Creating user {}", i + 1);
        match create_user(i as u32 + 1, *limit) {
            Ok(_) => {}
            Err(e) => {
                log!("Error creating user: {}", e);
                return;
            }
        }
    }
    log!("Database initialized successfully!");
}

pub fn read_user(id: u32) -> std::io::Result<User> {
    let file_path = format!("data/{}.bin", id);
    let file = match File::open(file_path) {
        Ok(file) => file,
        Err(e) => {
            log!("Error opening file: {}", e);
            return Err(e);
        }
    };
    match file.lock_shared() {
        Ok(_) => {}
        Err(e) => {
            log!("Error locking file for read: {}", e);
            return Err(e);
        }
    };
    let mut buff_reader = BufReader::new(file);

    let mut serialized_user = Vec::new();
    match buff_reader.read_to_end(&mut serialized_user) {
        Ok(_) => {}
        Err(e) => {
            log!("Error reading from file: {}", e);
            return Err(e);
        }
    }

    let user = match bincode::deserialize(&serialized_user) {
        Ok(user) => Ok(user),
        Err(e) => {
            log!("Error deserializing user: {}", e);
            Err(std::io::Error::new(std::io::ErrorKind::Other, e))
        }
    };

    return user;
}

pub fn create_user(id: u32, limit: u32) -> std::io::Result<()> {
    let file_path = format!("data/{}.bin", id);
    let file = match File::create(file_path) {
        Ok(file) => file,
        Err(e) => {
            log!("Error opening file: {}", e);
            return Err(e);
        }
    };
    match file.lock_exclusive() {
        Ok(_) => {}
        Err(e) => {
            log!("Error locking file for read: {}", e);
            return Err(e);
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
            log!("Error serializing user: {}", e);
            return Err(std::io::Error::new(std::io::ErrorKind::Other, e));
        }
    };
    match buff_writer.write_all(&serialized_user) {
        Ok(_) => {}
        Err(e) => {
            log!("Error writing to file: {}", e);
            return Err(e);
        }
    }

    buff_writer.flush()?;

    Ok(())
}

pub fn update_user_with_transaction(
    id: u32,
    transaction: &Transaction,
) -> std::io::Result<Option<User>> {
    // init file and buffers
    let file_path = format!("data/{}.bin", id);
    let file_result = std::fs::OpenOptions::new()
        .create(true)
        .write(true)
        .read(true)
        .open(file_path);
    let file = match file_result {
        Ok(file) => file,
        Err(e) => {
            log!("Error opening file: {}", e);
            return Err(e);
        }
    };

    let mut buff_reader = BufReader::new(&file);
    let mut buff_writer = BufWriter::new(&file);

    match file.lock_exclusive() {
        Ok(_) => {}
        Err(e) => {
            log!("Error locking file for read: {}", e);
            return Err(e);
        }
    };

    // read serialized user from file
    let mut serialized_user = Vec::new();
    match buff_reader.read_to_end(&mut serialized_user) {
        Ok(_) => {}
        Err(e) => {
            log!("Error reading from file: {}", e);
            return Err(e);
        }
    }

    // deserialize user
    let mut user: User = match bincode::deserialize(&serialized_user) {
        Ok(user) => user,
        Err(e) => {
            log!("Error deserializing user: {}", e);
            return Err(std::io::Error::new(std::io::ErrorKind::Other, e));
        }
    };

    // compute transaction and update user if it is valid
    match user.compute_transaction(transaction) {
        TransactionResult::Ok => {
            log!("Transaction computed successfully! Adding to list of transactions.");
            user.add_transaction(transaction);
        }
        // Return None if the transaction is invalid
        TransactionResult::LimitExceeded => {
            log!("Limit exceeded for user {}", id);
            return Ok(None);
        }
        TransactionResult::InvalidTransactionType => {
            log!("Invalid transaction type for user {}", id);
            return Ok(None);
        }
    };

    // serialize updated user
    let serialized_user = match bincode::serialize(&user) {
        Ok(serialized_user) => serialized_user,
        Err(e) => {
            log!("Error serializing user: {}", e);
            return Err(std::io::Error::new(std::io::ErrorKind::Other, e));
        }
    };

    // move to start of file
    match buff_reader.seek(std::io::SeekFrom::Start(0)) {
        Ok(_) => {}
        Err(e) => {
            log!("Error seeking to start of file: {}", e);
            return Err(e);
        }
    };
    // write updated user to file
    match buff_writer.write_all(&serialized_user) {
        Ok(_) => {}
        Err(e) => {
            log!("Error writing to file: {}", e);
            return Err(e);
        }
    }

    return Ok(Some(user));
}
