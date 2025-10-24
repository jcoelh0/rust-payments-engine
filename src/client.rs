use rust_decimal::prelude::*;
use std::collections::HashMap;

use crate::transaction::{Transaction, TransactionType};

pub struct Client {
    pub id: u16,
    pub available: Decimal,
    pub held: Decimal,
    pub total: Decimal,
    pub locked: bool,
    transactions: HashMap<u32, Transaction>,
    disputed_transactions: HashMap<u32, Decimal>,
}
impl Client {
    pub fn new(id: u16) -> Self {
        Client {
            id,
            available: dec!(0),
            held: dec!(0),
            total: dec!(0),
            locked: false,
            transactions: HashMap::new(), //only store deposits (only deposits can be disputed)
            disputed_transactions: HashMap::new(),
        }
    }

    pub fn deposit(&mut self, tx_id: u32, amount: Decimal) -> Result<(), String> {
        if self.locked {
            return Err("Account is locked".to_string());
        }
        self.available += amount;
        self.total += amount;
        self.transactions.insert(
            tx_id,
            Transaction {
                tx_type: TransactionType::Deposit,
                amount: Some(amount),
            },
        );
        Ok(())
    }

    pub fn withdraw(&mut self, amount: Decimal) -> Result<(), String> {
        if self.locked {
            return Err("Account is locked".to_string());
        }
        if self.available >= amount {
            self.available -= amount;
            self.total -= amount;
            Ok(())
        } else {
            Err("Insufficient available funds".to_string())
        }
    }

    pub fn dispute(&mut self, tx_id: u32) -> Result<(), String> {
        if self.locked {
            return Err("Account is locked".to_string());
        }
        if self.disputed_transactions.contains_key(&tx_id) {
            return Err("Transaction is already in dispute".to_string());
        }
        if let Some(tx) = self.transactions.get(&tx_id) {
            if tx.tx_type != TransactionType::Deposit {
                return Ok(());
            }
            if let Some(amount) = tx.amount {
                self.available -= amount;
                self.held += amount;
                self.disputed_transactions.insert(tx_id, amount);
            }
            Ok(())
        } else {
            Ok(())
        }
    }

    pub fn resolve(&mut self, tx_id: u32) -> Result<(), String> {
        if self.locked {
            return Err("Account is locked".to_string());
        }
        if let Some(amount) = self.disputed_transactions.get(&tx_id).cloned() {
            if self.held >= amount {
                self.held -= amount;
                self.available += amount;
                self.disputed_transactions.remove(&tx_id);
                Ok(())
            } else {
                Err("Insufficient held funds for resolve".to_string())
            }
        } else {
            Ok(())
        }
    }

    pub fn chargeback(&mut self, tx_id: u32) -> Result<(), String> {
        if self.locked {
            return Err("Account is already locked".to_string());
        }
        if let Some(amount) = self.disputed_transactions.get(&tx_id).cloned() {
            if self.held >= amount {
                self.held -= amount;
                self.total -= amount;
                self.locked = true;
                self.disputed_transactions.remove(&tx_id);
                Ok(())
            } else {
                Err("Insufficient held funds for chargeback".to_string())
            }
        } else {
            Ok(())
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn deposit_updates_balances_and_records_transaction() {
        let mut client = Client::new(1);
        client.deposit(1, dec!(10.5)).unwrap();

        assert_eq!(client.available, dec!(10.5));
        assert_eq!(client.total, dec!(10.5));
        assert_eq!(client.held, dec!(0));
        assert!(!client.locked);
        assert!(client.transactions.contains_key(&1));
    }

    #[test]
    fn withdraw_reduces_balances_when_sufficient_funds() {
        let mut client = Client::new(1);
        client.deposit(1, dec!(10)).unwrap();
        let result = client.withdraw(dec!(4));

        assert!(result.is_ok());
        assert_eq!(client.available, dec!(6));
        assert_eq!(client.total, dec!(6));
        assert_eq!(client.held, dec!(0));
    }

    #[test]
    fn withdraw_fails_when_insufficient_funds() {
        let mut client = Client::new(1);
        client.deposit(1, dec!(5)).unwrap();
        let result = client.withdraw(dec!(7));

        assert!(result.is_err());
        assert_eq!(client.available, dec!(5));
        assert_eq!(client.total, dec!(5));
    }

    #[test]
    fn dispute_moves_funds_from_available_to_held() {
        let mut client = Client::new(1);
        client.deposit(1, dec!(9)).unwrap();
        let result = client.dispute(1);

        assert!(result.is_ok());
        assert_eq!(client.available, dec!(0));
        assert_eq!(client.held, dec!(9));
        assert_eq!(client.total, dec!(9));
        assert!(client.disputed_transactions.contains_key(&1));
    }

    #[test]
    fn resolve_returns_disputed_funds_to_available() {
        let mut client = Client::new(1);
        client.deposit(1, dec!(8)).unwrap();
        client.dispute(1).unwrap();
        let result = client.resolve(1);

        assert!(result.is_ok());
        assert_eq!(client.available, dec!(8));
        assert_eq!(client.held, dec!(0));
        assert_eq!(client.total, dec!(8));
        assert!(!client.disputed_transactions.contains_key(&1));
    }

    #[test]
    fn dispute_on_unknown_transaction_is_ignored() {
        let mut client = Client::new(1);
        let result = client.dispute(999);

        assert!(result.is_ok());
        assert_eq!(client.available, dec!(0));
        assert_eq!(client.held, dec!(0));
        assert_eq!(client.total, dec!(0));
    }

    #[test]
    fn chargeback_deducts_total_and_locks_account() {
        let mut client = Client::new(1);
        client.deposit(1, dec!(12)).unwrap();
        client.dispute(1).unwrap();
        let result = client.chargeback(1);

        assert!(result.is_ok());
        assert_eq!(client.available, dec!(0));
        assert_eq!(client.held, dec!(0));
        assert_eq!(client.total, dec!(0));
        assert!(client.locked);
        assert!(!client.disputed_transactions.contains_key(&1));
    }

    #[test]
    fn chargeback_returns_error_when_account_already_locked() {
        let mut client = Client::new(1);
        client.deposit(1, dec!(10)).unwrap();
        client.dispute(1).unwrap();
        client.chargeback(1).unwrap();

        let result = client.chargeback(1);
        assert!(result.is_err());
        assert_eq!(result.unwrap_err(), "Account is already locked");
    }

    #[test]
    fn dispute_handles_insufficient_available_funds() {
        let mut client = Client::new(1);
        client.deposit(1, dec!(5)).unwrap();
        client.withdraw(dec!(4)).unwrap();

        let result = client.dispute(1);

        assert!(result.is_ok());
        assert_eq!(client.available, dec!(-4));
        assert_eq!(client.held, dec!(5));
        assert_eq!(client.total, dec!(1));
    }

    #[test]
    fn chargeback_returns_ok_when_transaction_not_in_dispute() {
        let mut client = Client::new(1);
        client.deposit(1, dec!(5)).unwrap();

        let result = client.chargeback(999);

        assert!(result.is_ok());
        assert_eq!(client.available, dec!(5));
        assert_eq!(client.total, dec!(5));
        assert!(!client.locked);
    }

    #[test]
    fn resolve_ignores_unknown_dispute() {
        let mut client = Client::new(1);
        let result = client.resolve(999);

        assert!(result.is_ok());
        assert_eq!(client.available, dec!(0));
        assert_eq!(client.held, dec!(0));
        assert_eq!(client.total, dec!(0));
    }

    #[test]
    fn dispute_returns_error_when_transaction_already_in_dispute() {
        let mut client = Client::new(1);
        client.deposit(1, dec!(6)).unwrap();
        client.dispute(1).unwrap();

        let result = client.dispute(1);

        assert!(result.is_err());
        assert_eq!(result.unwrap_err(), "Transaction is already in dispute");
    }
}
