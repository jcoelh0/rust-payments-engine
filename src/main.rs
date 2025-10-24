use client::Client;
use log::error;
use std::collections::BTreeMap;
use std::env;
use std::error::Error;
use std::fs::File;
use std::io::{self, BufReader, BufWriter};

use crate::transaction::TransactionType;
use crate::utils::{InputTransaction, format_decimal, missing_amount_error};

mod client;
mod transaction;
mod utils;

fn main() -> Result<(), Box<dyn Error>> {
    let args: Vec<String> = env::args().collect();
    if args.len() != 2 {
        return Err("Usage: cargo run -- <transactions.csv>".into());
    }

    let file = File::open(&args[1])?;
    let reader = BufReader::new(file);
    let stdout = io::stdout();
    let handle = stdout.lock();
    let writer = BufWriter::new(handle);

    process_transactions(reader, writer)
}

pub fn process_transactions<R: io::Read, W: io::Write>(
    source: R,
    writer: W,
) -> Result<(), Box<dyn Error>> {
    let mut reader = csv::Reader::from_reader(source);
    let mut clients: BTreeMap<u16, Client> = BTreeMap::new();

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

        let client = clients
            .entry(client_id)
            .or_insert_with(|| Client::new(client_id));

        match tx_type {
            TransactionType::Deposit => {
                if let None = amount {
                    return Err(missing_amount_error("deposit", tx).into());
                }
                if let Err(e) = client.deposit(tx, amount.unwrap()) {
                    error!("Error processing deposit: {}", e);
                }
            }
            TransactionType::Withdrawal => {
                if let None = amount {
                    return Err(missing_amount_error("withdrawal", tx).into());
                }
                if let Err(e) = client.withdraw(amount.unwrap()) {
                    error!("Error processing withdrawal: {}", e);
                }
            }
            TransactionType::Dispute => {
                if let Err(e) = client.dispute(tx) {
                    error!("Error processing dispute: {}", e);
                }
            }
            TransactionType::Resolve => {
                if let Err(e) = client.resolve(tx) {
                    error!("Error processing resolve: {}", e);
                }
            }
            TransactionType::Chargeback => {
                if let Err(e) = client.chargeback(tx) {
                    error!("Error processing chargeback: {}", e);
                }
            }
        }
    }

    let mut writer = csv::Writer::from_writer(writer);
    writer.write_record(&["client", "available", "held", "total", "locked"])?;

    for client in clients.values() {
        writer.write_record(&[
            client.id.to_string(),
            format_decimal(client.available),
            format_decimal(client.held),
            format_decimal(client.total),
            client.locked.to_string(),
        ])?;
    }

    writer.flush()?;
    Ok(())
}
