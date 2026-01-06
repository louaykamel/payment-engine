mod chargeback;
mod deposit;
mod dispute;
mod resolve;
mod withdrawal;

pub use chargeback::Chargeback;
pub use deposit::Deposit;
pub use dispute::Dispute;
pub use resolve::Resolve;
pub use withdrawal::Withdrawal;

use super::Decimal;
use crate::engine::error::TransactionError;
use serde::Deserialize;

pub type TransactionId = u32;

/// Raw transaction record as parsed from CSV input.
/// This is the unvalidated form that needs conversion to a specific Transaction type.
#[derive(Debug, Deserialize, Clone)]
pub struct TransactionRecord {
    #[serde(rename = "type")]
    pub tx_type: TransactionType,
    pub client: u16,
    /// Transaction ID (for Deposit/Withdrawal) or Reference ID (for Dispute/Resolve/Chargeback)
    pub tx: u32,
    /// Amount: required for Deposit/Withdrawal, must be None for Dispute/Resolve/Chargeback
    pub amount: Option<Decimal>,
}

impl std::fmt::Display for TransactionRecord {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self.amount {
            Some(amount) => write!(
                f,
                "{} (client: {}, tx: {}, amount: {})",
                self.tx_type, self.client, self.tx, amount
            ),
            None => write!(
                f,
                "{} (client: {}, tx: {})",
                self.tx_type, self.client, self.tx
            ),
        }
    }
}

#[derive(Debug, Deserialize, Clone, Copy)]
#[serde(rename_all = "lowercase")]
pub enum TransactionType {
    Deposit,
    Withdrawal,
    Dispute,
    Resolve,
    Chargeback,
}

impl std::fmt::Display for TransactionType {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            TransactionType::Deposit => write!(f, "deposit"),
            TransactionType::Withdrawal => write!(f, "withdrawal"),
            TransactionType::Dispute => write!(f, "dispute"),
            TransactionType::Resolve => write!(f, "resolve"),
            TransactionType::Chargeback => write!(f, "chargeback"),
        }
    }
}

/// A validated transaction ready for processing by the payment engine.
#[derive(Debug, Clone)]
pub enum Transaction {
    Deposit(Deposit),
    Withdrawal(Withdrawal),
    Dispute(Dispute),
    Resolve(Resolve),
    Chargeback(Chargeback),
}

impl TryFrom<TransactionRecord> for Transaction {
    type Error = TransactionError;

    fn try_from(record: TransactionRecord) -> Result<Self, Self::Error> {
        match record.tx_type {
            TransactionType::Deposit => Ok(Transaction::Deposit(Deposit::try_from(record)?)),
            TransactionType::Withdrawal => {
                Ok(Transaction::Withdrawal(Withdrawal::try_from(record)?))
            }
            TransactionType::Dispute => Ok(Transaction::Dispute(Dispute::try_from(record)?)),
            TransactionType::Resolve => Ok(Transaction::Resolve(Resolve::try_from(record)?)),
            TransactionType::Chargeback => {
                Ok(Transaction::Chargeback(Chargeback::try_from(record)?))
            }
        }
    }
}

impl std::fmt::Display for Transaction {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Transaction::Deposit(d) => {
                write!(
                    f,
                    "[deposit] client={} tx={} amount={}",
                    d.client_id(),
                    d.transaction_id(),
                    d.amount()
                )
            }
            Transaction::Withdrawal(w) => {
                write!(
                    f,
                    "[withdrawal] client={} amount={}",
                    w.client_id(),
                    w.amount()
                )
            }
            Transaction::Dispute(d) => {
                write!(
                    f,
                    "[dispute] client={} ref_tx={}",
                    d.client_id(),
                    d.referenced_tx_id()
                )
            }
            Transaction::Resolve(r) => {
                write!(
                    f,
                    "[resolve] client={} ref_tx={}",
                    r.client_id(),
                    r.referenced_tx_id()
                )
            }
            Transaction::Chargeback(c) => {
                write!(
                    f,
                    "[chargeback] client={} ref_tx={}",
                    c.client_id(),
                    c.referenced_tx_id()
                )
            }
        }
    }
}
