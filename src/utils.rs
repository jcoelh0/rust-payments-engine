use rust_decimal::Decimal;
use serde::Deserialize;
use std::error::Error;
use std::fmt;

use crate::transaction::TransactionType;

pub fn format_decimal(value: Decimal) -> String {
    format!("{:.4}", value)
}

#[derive(Deserialize)]
pub struct InputTransaction {
    #[serde(rename = "type")]
    pub tx_type: TransactionType,
    pub client: u16,
    pub tx: u32,
    pub amount: Option<Decimal>,
}

#[derive(Debug)]
pub struct MissingAmount {
    tx_type: &'static str,
    tx: u32,
}

impl MissingAmount {
    pub fn new(tx_type: &'static str, tx: u32) -> Self {
        Self { tx_type, tx }
    }
}

impl fmt::Display for MissingAmount {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(
            f,
            "Missing amount for {} transaction {}",
            self.tx_type, self.tx
        )
    }
}

impl Error for MissingAmount {}

pub fn missing_amount_error(tx_type: &'static str, tx: u32) -> MissingAmount {
    MissingAmount::new(tx_type, tx)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn format_decimal_pads_to_four_places() {
        let value = Decimal::new(123, 2); // 1.23
        assert_eq!(format_decimal(value), "1.2300");
    }

    #[test]
    fn missing_amount_helper_formats_message() {
        let err = missing_amount_error("deposit", 42);
        assert_eq!(err.to_string(), "Missing amount for deposit transaction 42");
    }
}
