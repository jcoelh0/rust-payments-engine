use rust_decimal::prelude::*;
use std::collections::HashMap;

use crate::errors::ClientTransactionError;

pub struct Client {
    pub id: u16,
    pub available: Decimal,
    pub held: Decimal,
    pub total: Decimal,
    pub locked: bool,
    deposit_transactions: HashMap<u32, Decimal>,
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
            deposit_transactions: HashMap::new(),
            disputed_transactions: HashMap::new(),
        }
    }

    pub fn deposit(&mut self, tx_id: u32, amount: Decimal) -> Result<(), ClientTransactionError> {
        if self.locked {
            return Err(ClientTransactionError::AccountLocked { client_id: self.id });
        }
        self.available += amount;
        self.total += amount;
        self.deposit_transactions.insert(tx_id, amount);
        Ok(())
    }

    pub fn withdraw(&mut self, amount: Decimal) -> Result<(), ClientTransactionError> {
        if self.locked {
            return Err(ClientTransactionError::AccountLocked { client_id: self.id });
        }
        if self.available < amount {
            return Err(ClientTransactionError::InsufficientAvailableFunds { client_id: self.id });
        }
        self.available -= amount;
        self.total -= amount;

        Ok(())
    }

    pub fn dispute(&mut self, tx_id: u32) -> Result<(), ClientTransactionError> {
        if self.locked {
            return Err(ClientTransactionError::AccountLocked { client_id: self.id });
        }
        if self.disputed_transactions.contains_key(&tx_id) {
            return Err(ClientTransactionError::AlreadyInDispute {
                client_id: self.id,
                tx_id,
            });
        }
        let amount = self.deposit_transactions.get(&tx_id).cloned().ok_or(
            ClientTransactionError::UnknownTransaction {
                client_id: self.id,
                tx_id,
            },
        )?;

        self.available -= amount;
        self.held += amount;
        self.disputed_transactions.insert(tx_id, amount);
        Ok(())
    }

    pub fn resolve(&mut self, tx_id: u32) -> Result<(), ClientTransactionError> {
        if self.locked {
            return Err(ClientTransactionError::AccountLocked { client_id: self.id });
        }
        let amount = self.disputed_transactions.get(&tx_id).cloned().ok_or(
            ClientTransactionError::NotInDispute {
                client_id: self.id,
                tx_id,
            },
        )?;

        if self.held < amount {
            return Err(ClientTransactionError::InsufficientHeldFunds {
                client_id: self.id,
                action: "resolve",
            });
        }

        self.held -= amount;
        self.available += amount;
        self.disputed_transactions.remove(&tx_id);
        Ok(())
    }

    pub fn chargeback(&mut self, tx_id: u32) -> Result<(), ClientTransactionError> {
        if self.locked {
            return Err(ClientTransactionError::AccountAlreadyLocked { client_id: self.id });
        }
        let amount = self.disputed_transactions.get(&tx_id).cloned().ok_or(
            ClientTransactionError::NotInDispute {
                client_id: self.id,
                tx_id,
            },
        )?;

        if self.held < amount {
            return Err(ClientTransactionError::InsufficientHeldFunds {
                client_id: self.id,
                action: "chargeback",
            });
        }

        self.held -= amount;
        self.total -= amount;
        self.locked = true;
        self.disputed_transactions.remove(&tx_id);
        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::errors::ClientTransactionError;

    #[test]
    fn deposit_updates_balances_and_records_transaction() {
        let mut client = Client::new(1);
        client.deposit(1, dec!(10.5)).unwrap();

        assert_eq!(client.available, dec!(10.5));
        assert_eq!(client.total, dec!(10.5));
        assert_eq!(client.held, dec!(0));
        assert!(!client.locked);
        assert!(client.deposit_transactions.contains_key(&1));
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

        assert!(matches!(
            result,
            Err(ClientTransactionError::InsufficientAvailableFunds { client_id: 1 })
        ));
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
    fn dispute_returns_error_when_transaction_unknown() {
        let mut client = Client::new(1);
        let result = client.dispute(999);

        assert!(matches!(
            result,
            Err(ClientTransactionError::UnknownTransaction {
                client_id: 1,
                tx_id: 999
            })
        ));
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
        assert!(matches!(
            result,
            Err(ClientTransactionError::AccountAlreadyLocked { client_id: 1 })
        ));
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
    fn chargeback_returns_error_when_transaction_not_in_dispute() {
        let mut client = Client::new(1);
        client.deposit(1, dec!(5)).unwrap();

        let result = client.chargeback(999);

        assert!(matches!(
            result,
            Err(ClientTransactionError::NotInDispute {
                client_id: 1,
                tx_id: 999
            })
        ));
    }

    #[test]
    fn resolve_returns_error_when_transaction_not_in_dispute() {
        let mut client = Client::new(1);
        let result = client.resolve(999);

        assert!(matches!(
            result,
            Err(ClientTransactionError::NotInDispute {
                client_id: 1,
                tx_id: 999
            })
        ));
    }

    #[test]
    fn dispute_returns_error_when_transaction_already_in_dispute() {
        let mut client = Client::new(1);
        client.deposit(1, dec!(6)).unwrap();
        client.dispute(1).unwrap();

        let result = client.dispute(1);

        assert!(matches!(
            result,
            Err(ClientTransactionError::AlreadyInDispute {
                client_id: 1,
                tx_id: 1
            })
        ));
    }
}
