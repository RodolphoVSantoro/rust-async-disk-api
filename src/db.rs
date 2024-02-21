use std::sync::Arc;

use sqlx::{Pool, Postgres};

use crate::logging;
use crate::transaction::{self, Transaction};
use crate::user::{TransactionResult, User, UserDb};

const INITIAL_USER_LIMITS: [i32; 5] = [100_000, 80_000, 1_000_000, 10_000_000, 500_000];
pub async fn reset(pool: &Pool<Postgres>) {
    logging::log!("Initializing database");
    let delete_result = sqlx::query("DELETE FROM users").execute(pool).await;
    match delete_result {
        Ok(rows) => {
            logging::log!("{} Users deleted successfully!", rows.rows_affected());
        }
        Err(e) => {
            panic!("Error deleting users: {}", e);
        }
    };
    for (i, limit) in INITIAL_USER_LIMITS.iter().enumerate() {
        let user = User {
            id: i32::try_from(i + 1).expect("Error converting user id"),
            balance_limit: *limit,
            balance: 0,
            transactions_count: 0,
            last_transaction: 0,
            transactions: Default::default(),
        };
        match create_user(pool, user).await {
            CreateUserResult::Ok(_) => {
                logging::log!("User {} created successfully!", i + 1);
            }
            CreateUserResult::InternalError(e) => {
                panic!("Error creating user {}: {}", i + 1, e);
            }
        };
    }
}

pub enum ReadUserResult {
    Ok,
    NotFound,
    InternalError(String),
}

pub async fn read_user(pool: Arc<Pool<Postgres>>, id: i32, user: &mut User) -> ReadUserResult {
    let db_user = match sqlx::query_as!(UserDb, "SELECT * FROM users WHERE id = $1", id)
        .fetch_optional(pool.as_ref())
        .await
    {
        Ok(user) => user,
        Err(e) => {
            let error_str = format!("Error reading user: {}", e);
            return ReadUserResult::InternalError(error_str);
        }
    };
    return match db_user {
        Some(db_user) => {
            user.balance = db_user.balance;
            user.balance_limit = db_user.balance_limit;
            user.transactions_count = db_user.transactions_count;
            user.last_transaction = db_user.last_transaction;
            transaction::decode_transactions(db_user.encoded_transactions, &mut user.transactions);
            return ReadUserResult::Ok;
        }
        None => ReadUserResult::NotFound,
    };
}
pub enum CreateUserResult {
    Ok(u64),
    InternalError(String),
}

pub async fn create_user(pool: &Pool<Postgres>, user: User) -> CreateUserResult {
    let insert_result = sqlx::query_as!(
            UserDb,
            "INSERT INTO users (id, balance_limit, balance, transactions_count, last_transaction, encoded_transactions) VALUES ($1, $2, $3, $4, $5, $6)",
            user.id,
            user.balance_limit,
            user.balance,
            user.transactions_count,
            user.last_transaction,
            transaction::encode_transactions(&user.transactions)
        ).execute(pool).await;
    return match insert_result {
        Ok(rows) => CreateUserResult::Ok(rows.rows_affected()),
        Err(e) => {
            let error_str = format!("Error inserting users: {}", e);
            return CreateUserResult::InternalError(error_str);
        }
    };
}

pub enum UpdateUserResult {
    Ok(User),
    NotFound,
    Unprocessable(String),
    InternalError(String),
}

pub async fn update_user_with_transaction(
    pool: Arc<Pool<Postgres>>,
    id: i32,
    transaction: &Transaction,
) -> UpdateUserResult {
    let postgres_transaction = pool.begin().await;
    let mut postgres_transaction = match postgres_transaction {
        Ok(transaction) => transaction,
        Err(e) => {
            let error_str = format!("Error starting transaction: {}", e);
            return UpdateUserResult::InternalError(error_str);
        }
    };
    let db_user = match sqlx::query_as!(UserDb, "SELECT * FROM users WHERE id = $1 FOR UPDATE", id)
        .fetch_optional(&mut *postgres_transaction)
        .await
    {
        Ok(Some(user)) => user,
        Ok(None) => {
            return UpdateUserResult::NotFound;
        }
        Err(e) => {
            let error_str = format!("Error reading user for update: {}", e);
            return UpdateUserResult::InternalError(error_str);
        }
    };

    let mut user = User::from(db_user);

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
    let update_result = sqlx::query!(
        "UPDATE users SET balance = $1, transactions_count = $2, last_transaction = $3, encoded_transactions = $4 WHERE id = $5",
        user.balance,
        user.transactions_count,
        user.last_transaction,
        transaction::encode_transactions(&user.transactions),
        id
    ).execute(&mut *postgres_transaction).await;

    match update_result {
        Ok(_) => {}
        Err(e) => {
            let error_string = format!("Error updating user: {}", e);
            return UpdateUserResult::InternalError(error_string);
        }
    };
    return match postgres_transaction.commit().await {
        Ok(()) => UpdateUserResult::Ok(user),
        Err(e) => {
            let error_string = format!("Error committing transaction: {}", e);
            return UpdateUserResult::InternalError(error_string);
        }
    };
}
