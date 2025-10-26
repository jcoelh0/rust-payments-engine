use std::env;
use std::fs::File;
use std::io::{BufReader, BufWriter};

use rust_payments_engine::errors::EngineError;
use rust_payments_engine::process_transactions;

fn main() -> Result<(), EngineError> {
    env_logger::init();
    let args: Vec<String> = env::args().collect();
    if args.len() != 2 {
        return Err(EngineError::Usage(
            "Usage: cargo run -- <transactions.csv>".to_string(),
        ));
    }

    let csv_file = File::open(&args[1])?;
    let reader = BufReader::new(csv_file);
    let stdout = std::io::stdout();
    let handle = stdout.lock();
    let writer = BufWriter::new(handle);

    process_transactions(reader, writer)
}
