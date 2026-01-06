use crate::engine::{
    error::TransactionError,
    transaction::{TransactionRecord, TransactionType},
};

/// A validated resolve transaction.
///
/// A resolve represents a resolution to a dispute, releasing the held funds.
/// Held funds decrease and available funds increase by the disputed amount.
/// Total funds remain the same.
/// Resolves reference the disputed transaction by ID and do not specify an amount.
#[derive(Debug, Clone)]
pub struct Resolve {
    client_id: u16,
    /// The ID of the transaction being resolved
    referenced_tx_id: u32,
}

impl Resolve {
    pub fn client_id(&self) -> u16 {
        self.client_id
    }

    pub fn referenced_tx_id(&self) -> u32 {
        self.referenced_tx_id
    }
}

impl TryFrom<TransactionRecord> for Resolve {
    type Error = TransactionError;

    fn try_from(record: TransactionRecord) -> Result<Self, Self::Error> {
        match record {
            TransactionRecord {
                tx_type: TransactionType::Resolve,
                client,
                tx,
                amount: None,
            } => Ok(Resolve {
                client_id: client,
                referenced_tx_id: tx,
            }),
            _ => Err(TransactionError::InvalidTransaction(record)),
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use rust_decimal_macros::dec;

    #[test]
    fn test_valid_resolve() {
        let record = TransactionRecord {
            tx_type: TransactionType::Resolve,
            client: 2,
            tx: 5,
            amount: None,
        };
        let resolve = Resolve::try_from(record).unwrap();

        assert_eq!(resolve.client_id(), 2);
        assert_eq!(resolve.referenced_tx_id(), 5);
    }

    #[test]
    fn test_rejects_with_amount() {
        let record = TransactionRecord {
            tx_type: TransactionType::Resolve,
            client: 1,
            tx: 5,
            amount: Some(dec!(100)),
        };
        assert!(Resolve::try_from(record).is_err());
    }

    #[test]
    fn test_rejects_wrong_transaction_type() {
        let record = TransactionRecord {
            tx_type: TransactionType::Dispute,
            client: 1,
            tx: 5,
            amount: None,
        };
        assert!(Resolve::try_from(record).is_err());
    }
}
