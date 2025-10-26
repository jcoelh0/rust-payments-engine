use rust_payments_engine::process_transactions;
use std::io::Cursor;

fn csv_lines(lines: &[&str]) -> String {
    let mut content = lines.join("\n");
    content.push('\n');
    content
}

fn get_output_from_raw_csv(csv: &str) -> String {
    let mut output = Vec::new();
    process_transactions(Cursor::new(csv.as_bytes()), &mut output)
        .expect("processing transactions");
    String::from_utf8(output).expect("csv writer produces utf-8")
}

#[test]
fn process_transactions_ignores_negative_transaction_ids() {
    let csv = csv_lines(&[
        "type,client,tx,amount",
        "deposit,1,-5,1.0",
        "deposit,1,0,2.0",
    ]);
    let output = get_output_from_raw_csv(&csv);
    assert!(output.contains("client,available,held,total,locked"));
    assert!(output.contains("1,2.0000,0.0000,2.0000,false"));
    assert!(!output.contains("1.0000"));
}

#[test]
fn process_transactions_accumulates_multiple_deposit_records() {
    let csv = csv_lines(&[
        "type,client,tx,amount",
        "deposit,1,1,5.0",
        "deposit,1,2,3.0",
    ]);
    let output = get_output_from_raw_csv(&csv);
    assert!(output.contains("client,available,held,total,locked"));
    assert!(output.contains("1,8.0000,0.0000,8.0000,false"));
}

#[test]
fn process_transactions_skips_non_positive_amount_rows() {
    let csv = csv_lines(&[
        "type,client,tx,amount",
        "deposit,1,1,-5.0",
        "deposit,1,2,3.0",
    ]);
    let output = get_output_from_raw_csv(&csv);
    assert!(output.contains("1,3.0000,0.0000,3.0000,false"));
    assert!(!output.contains("-5.0"));
}

#[test]
fn process_transactions_skips_rows_without_amount_for_deposits() {
    let csv = csv_lines(&["type,client,tx,amount", "deposit,1,1,"]);
    let output = get_output_from_raw_csv(&csv);
    assert_eq!(output, "client,available,held,total,locked\n");
}

#[test]
fn process_transactions_skips_rows_without_amount_for_withdrawals() {
    let csv = csv_lines(&[
        "type,client,tx,amount",
        "deposit,1,1,3.5",
        "withdrawal,1,2,",
    ]);
    let output = get_output_from_raw_csv(&csv);
    assert!(output.contains("client,available,held,total,locked"));
    assert!(output.contains("1,3.5000,0.0000,3.5000,false"));
    assert!(!output.contains(",2,"));
}

#[test]
fn process_transactions_preserves_balances_when_withdrawal_fails() {
    let csv = csv_lines(&[
        "type,client,tx,amount",
        "deposit,1,1,5.0",
        "withdrawal,1,2,10.0",
    ]);
    let output = get_output_from_raw_csv(&csv);
    assert!(output.contains("1,5.0000,0.0000,5.0000,false"));
    assert!(!output.contains("10.0000"));
}

#[test]
fn process_transactions_applies_dispute_and_resolve_flow() {
    let csv = csv_lines(&[
        "type,client,tx,amount",
        "deposit,1,1,2.5",
        "dispute,1,1,",
        "resolve,1,1,",
    ]);
    let output = get_output_from_raw_csv(&csv);
    assert!(output.contains("1,2.5000,0.0000,2.5000,false"));
}

#[test]
fn process_transactions_applies_dispute_and_chargeback_flow() {
    let csv = csv_lines(&[
        "type,client,tx,amount",
        "deposit,1,1,7.5",
        "dispute,1,1,",
        "chargeback,1,1,",
    ]);
    let output = get_output_from_raw_csv(&csv);
    assert!(output.contains("1,0.0000,0.0000,0.0000,true"));
}

#[test]
fn process_transactions_handles_duplicate_dispute_rows() {
    let csv = csv_lines(&[
        "type,client,tx,amount",
        "deposit,1,1,4.0",
        "deposit,1,2,4.0",
        "dispute,1,1,",
        "dispute,1,1,",
        "dispute,1,2,",
    ]);
    let output = get_output_from_raw_csv(&csv);
    assert!(output.contains("1,0.0000,8.0000,8.0000,false"));
}

#[test]
fn process_transactions_skips_transaction_ids_that_overflow_u32() {
    let csv = csv_lines(&[
        "type,client,tx,amount",
        "deposit,1,4294967296,1.0",
        "deposit,1,1,4.0",
    ]);
    let output = get_output_from_raw_csv(&csv);
    assert!(output.contains("1,4.0000,0.0000,4.0000,false"));
    assert!(!output.contains("4294967296"));
}
