use crate::transaction::TransactionType;
use rust_decimal::Decimal;
use thiserror::Error;

#[derive(Debug, Error, PartialEq, Eq)]
pub enum ClientTransactionError {
    #[error("Client {client_id}: account is locked")]
    AccountLocked { client_id: u16 },
    #[error("Client {client_id}: account is already locked")]
    AccountAlreadyLocked { client_id: u16 },
    #[error("Client {client_id}: invalid transaction id {tx}")]
    InvalidTransactionId { client_id: u16, tx: i64 },
    #[error("Client {client_id}: insufficient available funds")]
    InsufficientAvailableFunds { client_id: u16 },
    #[error("Client {client_id}: missing amount for {tx_type} transaction {tx}")]
    MissingAmount {
        client_id: u16,
        tx_type: TransactionType,
        tx: u32,
    },
    #[error("Client {client_id}: invalid amount {amount} for transaction {tx}")]
    InvalidAmount {
        client_id: u16,
        tx: u32,
        amount: Decimal,
    },
    #[error("Client {client_id}: insufficient held funds for {action}")]
    InsufficientHeldFunds {
        client_id: u16,
        action: &'static str,
    },
    #[error("Client {client_id}: transaction {tx_id} is unknown")]
    UnknownTransaction { client_id: u16, tx_id: u32 },
    #[error("Client {client_id}: transaction {tx_id} is already in dispute")]
    AlreadyInDispute { client_id: u16, tx_id: u32 },
    #[error("Client {client_id}: transaction {tx_id} is not under dispute")]
    NotInDispute { client_id: u16, tx_id: u32 },
}
