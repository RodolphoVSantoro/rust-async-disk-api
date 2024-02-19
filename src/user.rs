use crate::transaction::Transaction;
use serde::{Deserialize, Serialize};

#[derive(Serialize, Deserialize, Debug)]
pub struct User {
    pub limit: u32,
    pub total: i32,
    pub n_transactions: u32,
    pub last_transaction: u32,
    pub transactions: [Transaction; 10],
}

pub enum TransactionResult {
    Ok,
    LimitExceeded,
    InvalidTransactionType,
}

impl User {
    pub fn compute_transaction(&mut self, transaction: &Transaction) -> TransactionResult {
        if transaction.descricao.len() > 10 || transaction.descricao.is_empty() {
            return TransactionResult::InvalidTransactionType;
        }

        let int_transaction_value: i32 = transaction
            .valor
            .try_into()
            .expect("failed to convert transaction.valor to i32");
        match transaction.tipo.as_str() {
            "c" => {
                self.total += int_transaction_value;
                return TransactionResult::Ok;
            }
            "d" => {
                let limit: i32 = self
                    .limit
                    .try_into()
                    .expect("failed to convert limit to i32");
                if self.total - int_transaction_value < -limit {
                    return TransactionResult::LimitExceeded;
                }
                self.total -= int_transaction_value;
                return TransactionResult::Ok;
            }
            _ => return TransactionResult::InvalidTransactionType,
        }
    }
    pub fn add_transaction(&mut self, transaction: &Transaction) {
        let copy_transaction = Transaction {
            valor: transaction.valor,
            descricao: transaction.descricao.clone(),
            tipo: transaction.tipo.clone(),
            realizada_em: transaction.realizada_em.clone(),
        };
        if self.n_transactions < 10 {
            let index: usize = self
                .n_transactions
                .try_into()
                .expect("failed to convert n_transactions to usize");
            self.transactions[index] = copy_transaction;
            self.n_transactions += 1;
            self.last_transaction = self.n_transactions % 10;
            return;
        }
        let index: usize = self
            .last_transaction
            .try_into()
            .expect("failed to convert n_transactions to usize");
        self.transactions[index] = copy_transaction;
        self.last_transaction = (self.last_transaction + 1) % 10;
    }
    pub fn get_ordered_transactions(&self) -> Vec<&Transaction> {
        if self.n_transactions == 0 {
            return Vec::new();
        }
        let mut ordered_transactions = Vec::new();
        let n_transactions: i32 = self
            .n_transactions
            .try_into()
            .expect("failed to convert n_transactions to i32");
        let mut i: i32 = self
            .last_transaction
            .try_into()
            .expect("failed to convert last_transaction to i32");

        i = (i - 1 + n_transactions) % n_transactions;
        for _ in 0..n_transactions {
            let index: usize = i.try_into().expect("failed to convert i to i32");
            ordered_transactions.push(&self.transactions[index]);
            i = (i - 1 + n_transactions) % n_transactions;
        }
        return ordered_transactions;
    }
}
