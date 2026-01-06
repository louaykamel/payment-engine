//! Basic example of using the `PaymentEngine`.
//!
//! Run with: `cargo run --example basic`

use payment_engine::PaymentEngine;
use std::io::Cursor;

fn main() {
    // Initialize logger (optional, but shows what's happening)
    env_logger::Builder::from_env(env_logger::Env::default().default_filter_or("info")).init();

    // Sample transactions as CSV
    let transactions = r"type,client,tx,amount
deposit,1,1,100.0
deposit,2,2,200.0
deposit,1,3,50.0
withdrawal,1,4,30.0
dispute,2,2,
resolve,2,2,
deposit,1,5,25.0
withdrawal,2,6,50.0
dispute,1,1,
chargeback,1,1,
";

    // Create engine and process transactions
    let mut engine = PaymentEngine::new();
    engine
        .process_transactions(Cursor::new(transactions))
        .expect("Failed to process transactions");

    // Export results to stdout
    println!("\n=== Final Account State ===");
    engine
        .export_accounts(std::io::stdout())
        .expect("Failed to export accounts");
}
