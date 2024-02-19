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
        if transaction.descricao.len() > 10 || transaction.descricao.len() < 1 {
            return TransactionResult::InvalidTransactionType;
        }

        let int_transaction_value = transaction.valor as i32;
        match transaction.tipo.as_str() {
            "c" => {
                self.total += int_transaction_value;
                return TransactionResult::Ok;
            }
            "d" => {
                if self.total - int_transaction_value < -(self.limit as i32) {
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
            self.transactions[self.n_transactions as usize] = copy_transaction;
            self.n_transactions += 1;
            self.last_transaction = self.n_transactions;
            return;
        }
        self.last_transaction = (self.last_transaction + 1) % 10;
        self.transactions[self.last_transaction as usize] = copy_transaction;
    }
    pub fn get_ordered_transactions(&self) -> Vec<&Transaction> {
        if self.n_transactions == 0 {
            return Vec::new();
        }
        let mut ordered_transactions = Vec::new();
        let mut i = self.last_transaction as i32;
        i = (i - 1 + self.n_transactions as i32) % self.n_transactions as i32;
        for _ in 0..self.n_transactions {
            ordered_transactions.push(&self.transactions[i as usize]);
            i = (i - 1 + self.n_transactions as i32) % self.n_transactions as i32;
        }
        return ordered_transactions;
    }
}
