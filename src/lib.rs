pub mod client;
pub mod errors;
pub mod transaction;

use client::Client;
use errors::{ClientTransactionError, EngineError};
use log::error;
use rust_decimal::Decimal;
use serde::Deserialize;
use std::{
    collections::HashMap,
    io::{Read, Write},
};

use crate::transaction::TransactionType;

#[derive(Deserialize)]
struct InputTransaction {
    #[serde(rename = "type")]
    tx_type: TransactionType,
    client: u16,
    tx: i64,
    amount: Option<Decimal>,
}

pub fn format_decimal(value: Decimal) -> String {
    format!("{:.4}", value)
}

enum ValidatedTransaction {
    WithAmount { tx: u32, amount: Decimal },
    NoAmount { tx: u32 },
}

fn validate_transaction(
    tx_type: TransactionType,
    client_id: u16,
    tx: i64,
    amount: Option<Decimal>,
) -> Result<ValidatedTransaction, ClientTransactionError> {
    if tx < 0 {
        return Err(ClientTransactionError::InvalidTransactionId { client_id, tx });
    }

    let tx_u32 = u32::try_from(tx)
        .map_err(|_| ClientTransactionError::InvalidTransactionId { client_id, tx })?;

    match tx_type {
        TransactionType::Deposit | TransactionType::Withdrawal => match amount {
            Some(value) if value > Decimal::ZERO => Ok(ValidatedTransaction::WithAmount {
                tx: tx_u32,
                amount: value,
            }),
            Some(value) => Err(ClientTransactionError::InvalidAmount {
                client_id,
                tx: tx_u32,
                amount: value,
            }),
            None => Err(ClientTransactionError::MissingAmount {
                client_id,
                tx_type,
                tx: tx_u32,
            }),
        },
        _ => Ok(ValidatedTransaction::NoAmount { tx: tx_u32 }),
    }
}

pub fn process_transactions<R: Read, W: Write>(source: R, writer: W) -> Result<(), EngineError> {
    use transaction::TransactionType;
    let mut reader = csv::Reader::from_reader(source);
    let mut clients: HashMap<u16, Client> = HashMap::new();

    for (row_index, result) in reader.deserialize().enumerate() {
        let transaction: InputTransaction = match result {
            Ok(record) => record,
            Err(err) => {
                error!("Error parsing CSV row {}: {}", row_index + 1, err);
                continue;
            }
        };

        let InputTransaction {
            tx_type,
            client: client_id,
            tx,
            amount,
        } = transaction;

        let validated = match validate_transaction(tx_type, client_id, tx, amount) {
            Ok(value) => value,
            Err(err) => {
                error!("{}", err);
                continue;
            }
        };

        let client = clients
            .entry(client_id)
            .or_insert_with(|| Client::new(client_id));
        match (tx_type, validated) {
            (TransactionType::Deposit, ValidatedTransaction::WithAmount { tx, amount }) => {
                if let Err(e) = client.deposit(tx, amount) {
                    error!("Error processing deposit: {}", e);
                }
            }
            (TransactionType::Withdrawal, ValidatedTransaction::WithAmount { tx: _, amount }) => {
                if let Err(e) = client.withdraw(amount) {
                    error!("Error processing withdrawal: {}", e);
                }
            }
            (TransactionType::Dispute, ValidatedTransaction::NoAmount { tx }) => {
                if let Err(e) = client.dispute(tx) {
                    error!("Partner's error processing dispute: {}", e);
                }
            }
            (TransactionType::Resolve, ValidatedTransaction::NoAmount { tx }) => {
                if let Err(e) = client.resolve(tx) {
                    error!("Partner's error processing resolve: {}", e);
                }
            }
            (TransactionType::Chargeback, ValidatedTransaction::NoAmount { tx }) => {
                if let Err(e) = client.chargeback(tx) {
                    error!("Partner's error processing chargeback: {}", e);
                }
            }
            (tx_type, _) => {
                error!(
                    "Validation mismatch for client {} on transaction type {}",
                    client_id, tx_type
                );
            }
        }
    }

    let mut csv_writer = csv::Writer::from_writer(writer);
    csv_writer.write_record(&["client", "available", "held", "total", "locked"])?;

    let mut clients_sorted: Vec<&Client> = clients.values().collect();
    clients_sorted.sort_by_key(|client| client.id);

    for client in clients_sorted {
        csv_writer.write_record(&[
            client.id.to_string(),
            format_decimal(client.available),
            format_decimal(client.held),
            format_decimal(client.total),
            client.locked.to_string(),
        ])?;
    }

    csv_writer.flush()?;
    Ok(())
}
