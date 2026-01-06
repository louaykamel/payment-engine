use crate::engine::transaction::TransactionRecord;
use crate::engine::Decimal;

/// Top-level error type for the payment engine.
#[derive(Debug, thiserror::Error)]
#[allow(clippy::enum_variant_names)]
pub enum Error {
    #[error("IO error: {0}")]
    Io(#[from] std::io::Error),
    #[error("CSV error: {0}")]
    Csv(#[from] csv::Error),
    #[error("Transaction error: {0}")]
    Transaction(#[from] TransactionError),
}

/// Errors during `TransactionRecord` -> `Transaction` conversion (hard errors).
#[derive(Debug, thiserror::Error)]
pub enum TransactionError {
    #[error("Invalid transaction: {0}")]
    InvalidTransaction(TransactionRecord),
}

/// Soft (clients/partners) errors during transaction processing.
/// These don't stop batch processing, we log and continue.
#[derive(Debug, thiserror::Error)]
pub enum ProcessingError {
    #[error("Transaction {tx} not found")]
    TransactionNotFound { tx: u32 },

    #[error("Client mismatch: transaction {tx} belongs to client {expected}, not {got}")]
    ClientMismatch { tx: u32, expected: u16, got: u16 },

    #[error("Transaction {tx} is not under dispute")]
    NotUnderDispute { tx: u32 },

    #[error("Transaction {tx} is already under dispute")]
    AlreadyUnderDispute { tx: u32 },

    #[error("Insufficient funds: client {client} has {available}, requested {requested}")]
    InsufficientFunds {
        client: u16,
        available: Decimal,
        requested: Decimal,
    },

    #[error("Account {client} not found")]
    AccountNotFound { client: u16 },

    #[error("Account {client} is locked")]
    AccountLocked { client: u16 },
}
