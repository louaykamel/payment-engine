mod commands;

use anyhow::{Context, Result};
use clap::Parser;
use commands::Args;
use payment_engine::PaymentEngine;

fn main() -> Result<()> {
    // Parse the CLI arguments
    let args = Args::parse();

    // Initialize logger with default level of warn (can be overridden with RUST_LOG)
    env_logger::Builder::from_env(env_logger::Env::default().default_filter_or("info")).init();

    // 1. Initialize the PaymentEngine
    let mut engine = PaymentEngine::new();

    // 2. Open and process the input file
    log::info!("Processing transactions from {}", args.input_file.display());
    let file = std::fs::File::open(&args.input_file)
        .with_context(|| format!("Failed to open input file: {}", args.input_file.display()))?;

    engine
        .process_transactions(file)
        .context("Failed to process transactions")?;

    log::info!(
        "Processing complete, exporting {} accounts",
        engine.account_count()
    );

    // 3. Export the accounts to stdout
    engine
        .export_accounts(std::io::stdout())
        .context("Failed to export accounts to stdout")?;

    log::info!("Export complete");

    Ok(())
}
