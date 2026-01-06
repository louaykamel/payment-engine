//! Integration tests for the `PaymentEngine`.
//!
//! These tests exercise the full E2E flow: CSV input → processing → CSV output.
use payment_engine::{Account, PaymentEngine};
use rust_decimal_macros::dec;
use std::io::Cursor;

/// Helper to run a transaction CSV through the engine and get output
fn process_csv(input: &str) -> String {
    let mut engine = PaymentEngine::new();
    let reader = Cursor::new(input);
    engine.process_transactions(reader).unwrap();

    let mut output = Vec::new();
    engine.export_accounts(&mut output).unwrap();
    String::from_utf8(output).unwrap()
}

/// Parse CSV output into a vec of (client, available, held, total, locked)
fn parse_output(output: &str) -> Vec<Account> {
    let mut rdr = csv::Reader::from_reader(output.as_bytes());
    rdr.deserialize::<Account>().map(|r| r.unwrap()).collect()
}

#[test]
fn test_basic_deposit() {
    let input = "type,client,tx,amount
deposit,1,1,100.0";

    let output = process_csv(input);
    let accounts = parse_output(&output);

    assert_eq!(accounts.len(), 1);
    assert_eq!(accounts[0].client_id(), 1); // client
    assert_eq!(accounts[0].available(), dec!(100)); // available
    assert_eq!(accounts[0].held(), dec!(0)); // held
    assert_eq!(accounts[0].total(), dec!(100)); // total
    assert!(!accounts[0].is_locked()); // not locked
}

#[test]
fn test_deposit_and_withdrawal() {
    let input = "type,client,tx,amount
deposit,1,1,100.0
withdrawal,1,2,40.0";

    let output = process_csv(input);
    let accounts = parse_output(&output);

    assert_eq!(accounts.len(), 1);
    assert_eq!(accounts[0].available(), dec!(60)); // available
    assert_eq!(accounts[0].total(), dec!(60)); // total
}

#[test]
fn test_withdrawal_insufficient_funds_is_skipped() {
    let input = "type,client,tx,amount
deposit,1,1,50.0
withdrawal,1,2,100";

    let output = process_csv(input);
    let accounts = parse_output(&output);

    // Withdrawal should be skipped due to insufficient funds
    assert_eq!(accounts[0].available(), dec!(50));
    assert_eq!(accounts[0].total(), dec!(50));
}

#[test]
fn test_dispute_holds_funds() {
    let input = "type,client,tx,amount
deposit,1,1,100.0
dispute,1,1,";

    let output = process_csv(input);
    let accounts = parse_output(&output);

    assert_eq!(accounts[0].available(), dec!(0)); // available (held)
    assert_eq!(accounts[0].held(), dec!(100)); // held
    assert_eq!(accounts[0].total(), dec!(100)); // total unchanged
}

#[test]
fn test_resolve_releases_held_funds() {
    let input = "type,client,tx,amount
deposit,1,1,100.0
dispute,1,1,
resolve,1,1,";

    let output = process_csv(input);
    let accounts = parse_output(&output);

    assert_eq!(accounts[0].available(), dec!(100)); // available (released)
    assert_eq!(accounts[0].held(), dec!(0)); // held
    assert_eq!(accounts[0].total(), dec!(100)); // total unchanged
}

#[test]
fn test_chargeback_removes_funds_and_locks_account() {
    let input = "type,client,tx,amount
deposit,1,1,100.0
dispute,1,1,
chargeback,1,1,";

    let output = process_csv(input);
    let accounts = parse_output(&output);

    assert_eq!(accounts[0].available(), dec!(0)); // available
    assert_eq!(accounts[0].held(), dec!(0)); // held (removed)
    assert_eq!(accounts[0].total(), dec!(0)); // total reduced
    assert!(accounts[0].is_locked()); // locked!
}

#[test]
fn test_locked_account_ignores_deposits() {
    let input = "type,client,tx,amount
deposit,1,1,100.0
dispute,1,1,
chargeback,1,1,
deposit,1,2,500.0";

    let output = process_csv(input);
    let accounts = parse_output(&output);

    // Second deposit should be ignored because account is locked
    assert_eq!(accounts[0].total(), dec!(0));
    assert!(accounts[0].is_locked());
}

#[test]
fn test_dispute_wrong_client_is_ignored() {
    let input = "type,client,tx,amount
deposit,1,1,100.0
dispute,2,1,";

    let output = process_csv(input);
    let accounts = parse_output(&output);

    // Dispute from wrong client should be ignored
    assert_eq!(accounts[0].available(), dec!(100)); // still available
    assert_eq!(accounts[0].held(), dec!(0)); // not held
}

#[test]
fn test_resolve_without_dispute_is_ignored() {
    let input = "type,client,tx,amount
deposit,1,1,100.0
resolve,1,1,";

    let output = process_csv(input);
    let accounts = parse_output(&output);

    // Resolve without dispute should be ignored
    assert_eq!(accounts[0].available(), dec!(100));
    assert_eq!(accounts[0].held(), dec!(0));
}

#[test]
fn test_multiple_clients() {
    let input = "type,client,tx,amount
deposit,1,1,100.0
deposit,2,2,200.0
withdrawal,1,3,30.0";

    let output = process_csv(input);
    let accounts = parse_output(&output);

    assert_eq!(accounts.len(), 2);
    // Note: order may vary, so find by client id
    let client1 = accounts.iter().find(|a| a.client_id() == 1).unwrap();
    let client2 = accounts.iter().find(|a| a.client_id() == 2).unwrap();

    assert_eq!(client1.available(), dec!(70));
    assert_eq!(client2.available(), dec!(200));
}

#[test]
fn test_precision_handling() {
    let input = "type,client,tx,amount
deposit,1,1,1.2345
deposit,1,2,0.0001";

    let output = process_csv(input);
    let accounts = parse_output(&output);

    assert_eq!(accounts[0].available(), dec!(1.2346));
}

// ============================================================================
// Invalid Input Tests - These should cause errors
// ============================================================================

/// Helper that returns Result to test error cases
fn try_process_csv(input: &str) -> Result<String, Box<dyn std::error::Error>> {
    let mut engine = PaymentEngine::new();
    let reader = Cursor::new(input);
    engine.process_transactions(reader)?;

    let mut output = Vec::new();
    engine.export_accounts(&mut output)?;
    Ok(String::from_utf8(output)?)
}

#[test]
fn test_rejects_negative_deposit() {
    let input = "type,client,tx,amount
deposit,1,1,-100.0";

    // Negative amount should cause an error
    assert!(try_process_csv(input).is_err());
}

#[test]
fn test_rejects_negative_withdrawal() {
    let input = "type,client,tx,amount
deposit,1,1,100.0
withdrawal,1,2,-50.0";

    // Negative withdrawal should cause an error
    assert!(try_process_csv(input).is_err());
}

#[test]
fn test_rejects_more_than_4_decimals() {
    let input = "type,client,tx,amount
deposit,1,1,1.23456";

    // More than 4 decimal places should cause an error
    assert!(try_process_csv(input).is_err());
}

#[test]
fn test_rejects_zero_deposit() {
    let input = "type,client,tx,amount
deposit,1,1,0";

    // Zero amount should cause an error
    assert!(try_process_csv(input).is_err());
}

#[test]
fn test_rejects_zero_withdrawal() {
    let input = "type,client,tx,amount
deposit,1,1,100.0
withdrawal,1,2,0";

    // Zero withdrawal should cause an error
    assert!(try_process_csv(input).is_err());
}

#[test]
fn test_accepts_valid_precision_variants() {
    // All of these should be valid
    let inputs = [
        "type,client,tx,amount\ndeposit,1,1,100",
        "type,client,tx,amount\ndeposit,1,1,100.0",
        "type,client,tx,amount\ndeposit,1,1,100.00",
        "type,client,tx,amount\ndeposit,1,1,100.000",
        "type,client,tx,amount\ndeposit,1,1,100.0000",
        "type,client,tx,amount\ndeposit,1,1,0.0001",
    ];

    for input in inputs {
        assert!(try_process_csv(input).is_ok(), "Should accept: {input}");
    }
}

#[test]
fn test_whitespace_handling() {
    let input = "type,  client,  tx,  amount
deposit,  1,  1,  100.0";

    let output = process_csv(input);
    let accounts = parse_output(&output);

    assert_eq!(accounts[0].available(), dec!(100));
}

// ============================================================================
// Advanced Edge Cases
// ============================================================================

#[test]
fn test_dispute_after_partial_withdrawal_allows_negative_available() {
    let input = "type,client,tx,amount
deposit,1,1,100.0
withdrawal,1,2,80.0
dispute,1,1,";

    let output = process_csv(input);
    let accounts = parse_output(&output);

    // available = 20 - 100 = -80 (negative is allowed per spec)
    assert_eq!(accounts[0].available(), dec!(-80));
    assert_eq!(accounts[0].held(), dec!(100)); // held
    assert_eq!(accounts[0].total(), dec!(20)); // total unchanged
}

#[test]
fn test_double_dispute_same_transaction_is_ignored() {
    let input = "type,client,tx,amount
deposit,1,1,100.0
dispute,1,1,
dispute,1,1,";

    let output = process_csv(input);
    let accounts = parse_output(&output);

    // Second dispute should be ignored (already under dispute)
    assert_eq!(accounts[0].available(), dec!(0));
    assert_eq!(accounts[0].held(), dec!(100)); // held once, not twice
}

#[test]
fn test_chargeback_without_dispute_is_ignored() {
    let input = "type,client,tx,amount
deposit,1,1,100.0
chargeback,1,1,";

    let output = process_csv(input);
    let accounts = parse_output(&output);

    // Chargeback without dispute should be ignored
    assert_eq!(accounts[0].available(), dec!(100));
    assert_eq!(accounts[0].total(), dec!(100));
    assert!(!accounts[0].is_locked()); // not locked
}

#[test]
fn test_resolve_then_dispute_again() {
    let input = "type,client,tx,amount
deposit,1,1,100.0
dispute,1,1,
resolve,1,1,
dispute,1,1,";

    let output = process_csv(input);
    let accounts = parse_output(&output);

    // Can dispute again after resolve
    assert_eq!(accounts[0].available(), dec!(0));
    assert_eq!(accounts[0].held(), dec!(100));
}

#[test]
fn test_resolve_before_dispute_is_ignored() {
    let input = "type,client,tx,amount
deposit,1,1,100.0
resolve,1,1,";

    let output = process_csv(input);
    let accounts = parse_output(&output);

    // Resolve without dispute should be ignored
    assert_eq!(accounts[0].available(), dec!(100));
    assert_eq!(accounts[0].total(), dec!(100));
    assert!(!accounts[0].is_locked()); // not locked
}

#[test]
fn test_chargeback_on_different_client_transaction_is_ignored() {
    let input = "type,client,tx,amount
deposit,1,1,100.0
dispute,1,1,
chargeback,2,1,";

    let output = process_csv(input);
    let accounts = parse_output(&output);

    // Chargeback from wrong client is ignored
    assert_eq!(accounts[0].available(), dec!(0));
    assert_eq!(accounts[0].held(), dec!(100));
    assert!(!accounts[0].is_locked()); // not locked
}

#[test]
fn test_dispute_nonexistent_transaction_is_ignored() {
    let input = "type,client,tx,amount
deposit,1,1,100.0
dispute,1,999,";

    let output = process_csv(input);
    let accounts = parse_output(&output);

    // Dispute on non-existent tx is ignored
    assert_eq!(accounts[0].available(), dec!(100));
    assert_eq!(accounts[0].held(), dec!(0));
}

#[test]
fn test_withdrawal_from_nonexistent_client_is_ignored() {
    let input = "type,client,tx,amount
withdrawal,1,1,100.0";

    let output = process_csv(input);
    let accounts = parse_output(&output);

    // No accounts created since withdrawal failed
    assert_eq!(accounts.len(), 0);
}

#[test]
fn test_multiple_deposits_same_client() {
    let input = "type,client,tx,amount
deposit,1,1,100.0
deposit,1,2,50.5
deposit,1,3,25.25";

    let output = process_csv(input);
    let accounts = parse_output(&output);

    assert_eq!(accounts[0].available(), dec!(175.75));
}

#[test]
fn test_complete_dispute_flow() {
    // Deposit -> Dispute -> Resolve -> Dispute again -> Chargeback
    let input = "type,client,tx,amount
deposit,1,1,100.0
dispute,1,1,
resolve,1,1,
dispute,1,1,
chargeback,1,1,";

    let output = process_csv(input);
    let accounts = parse_output(&output);

    assert_eq!(accounts[0].available(), dec!(0));
    assert_eq!(accounts[0].held(), dec!(0));
    assert_eq!(accounts[0].total(), dec!(0));
    assert!(accounts[0].is_locked()); // locked
}

#[test]
fn test_multiple_disputes_different_transactions() {
    let input = "type,client,tx,amount
deposit,1,1,100.0
deposit,1,2,50.0
dispute,1,1,
dispute,1,2,";

    let output = process_csv(input);
    let accounts = parse_output(&output);

    assert_eq!(accounts[0].available(), dec!(0));
    assert_eq!(accounts[0].held(), dec!(150)); // both held
    assert_eq!(accounts[0].total(), dec!(150));
}

#[test]
fn test_partial_chargeback_flow() {
    let input = "type,client,tx,amount
deposit,1,1,100.0
deposit,1,2,50.0
dispute,1,1,
chargeback,1,1,";

    let output = process_csv(input);
    let accounts = parse_output(&output);

    // Only first deposit charged back, second still available (but account locked)
    assert_eq!(accounts[0].available(), dec!(50));
    assert_eq!(accounts[0].held(), dec!(0));
    assert_eq!(accounts[0].total(), dec!(50));
    assert!(accounts[0].is_locked()); // locked
}
