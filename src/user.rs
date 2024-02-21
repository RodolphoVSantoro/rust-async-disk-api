use crate::transaction::{self, Transaction};
use serde::{Deserialize, Serialize};

#[derive(Serialize, Deserialize, Debug)]
pub struct UserDb {
    pub id: i32,
    pub balance_limit: i32,
    pub balance: i32,
    pub transactions_count: i32,
    pub last_transaction: i32,
    pub encoded_transactions: Option<Vec<u8>>,
}

#[derive(Serialize, Deserialize, Debug)]
pub struct User {
    pub id: i32,
    pub balance_limit: i32,
    pub balance: i32,
    pub transactions_count: i32,
    pub last_transaction: i32,
    pub transactions: [Transaction; 10],
}

impl From<User> for UserDb {
    fn from(user: User) -> Self {
        return UserDb {
            id: user.id,
            balance_limit: user.balance_limit,
            balance: user.balance,
            transactions_count: user.transactions_count,
            last_transaction: user.last_transaction,
            encoded_transactions: Some(transaction::encode_transactions(&user.transactions)),
        };
    }
}

impl From<UserDb> for User {
    fn from(user: UserDb) -> Self {
        let mut transactions: [Transaction; 10] = Default::default();
        transaction::decode_transactions(user.encoded_transactions, &mut transactions);
        return User {
            id: user.id,
            balance_limit: user.balance_limit,
            balance: user.balance,
            transactions_count: user.transactions_count,
            last_transaction: user.last_transaction,
            transactions,
        };
    }
}

pub enum TransactionResult {
    Ok,
    LimitExceeded,
    InvalidTransactionKind(u8),
    InvalidDescription,
}

impl User {
    pub fn compute_transaction(&mut self, transaction: &Transaction) -> TransactionResult {
        if transaction.descricao.len() > 10 || transaction.descricao.is_empty() {
            return TransactionResult::InvalidDescription;
        }

        match transaction.tipo.as_str() {
            "c" => {
                self.balance += transaction.valor;
                return TransactionResult::Ok;
            }
            "d" => {
                let limit = self.balance_limit;
                if self.balance - transaction.valor < -limit {
                    return TransactionResult::LimitExceeded;
                }
                self.balance -= transaction.valor;
                return TransactionResult::Ok;
            }
            tipo => {
                let t = tipo.as_bytes()[0];
                return TransactionResult::InvalidTransactionKind(t);
            }
        }
    }
    pub fn add_transaction(&mut self, transaction: &Transaction) {
        let copy_transaction = Transaction {
            valor: transaction.valor,
            descricao: transaction.descricao.clone(),
            tipo: transaction.tipo.clone(),
            realizada_em: transaction.realizada_em.clone(),
        };
        if self.transactions_count < 10 {
            let index: usize = self
                .transactions_count
                .try_into()
                .expect("failed to convert n_transactions to usize");
            self.transactions[index] = copy_transaction;
            self.transactions_count += 1;
            self.last_transaction = self.transactions_count % 10;
            return;
        }
        let index: usize = self
            .last_transaction
            .try_into()
            .expect("failed to convert n_transactions to usize");
        self.transactions[index] = copy_transaction;
        self.last_transaction = (self.last_transaction + 1) % 10;
    }
    pub fn get_ordered_transactions<'a>(
        &'a self,
        ordered_transactions: &mut [Option<&'a Transaction>; 10],
    ) {
        if self.transactions_count == 0 {
            return;
        }
        let n_transactions = self.transactions_count;
        let mut i = self.last_transaction;
        i = (i - 1 + n_transactions) % n_transactions;
        for j in 0..n_transactions {
            let index_i: usize = i.try_into().expect("failed to convert i to i32");
            let index_j: usize = j.try_into().expect("failed to convert j to i32");

            ordered_transactions[index_j] = Some(&self.transactions[index_i]);
            i = (i - 1 + n_transactions) % n_transactions;
        }
    }
}
