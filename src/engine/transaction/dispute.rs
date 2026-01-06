use crate::engine::{
    error::TransactionError,
    transaction::{TransactionRecord, TransactionType},
};

/// A validated dispute transaction.
///
/// A dispute represents a client's claim that a transaction was erroneous.
/// The disputed funds are held (moved from available to held) while total remains the same.
/// Disputes reference the original transaction by ID and do not specify an amount.
#[derive(Debug, Clone)]
pub struct Dispute {
    client_id: u16,
    /// The ID of the transaction being disputed
    referenced_tx_id: u32,
}

impl Dispute {
    pub fn client_id(&self) -> u16 {
        self.client_id
    }

    pub fn referenced_tx_id(&self) -> u32 {
        self.referenced_tx_id
    }
}

impl TryFrom<TransactionRecord> for Dispute {
    type Error = TransactionError;

    fn try_from(record: TransactionRecord) -> Result<Self, Self::Error> {
        match record {
            TransactionRecord {
                tx_type: TransactionType::Dispute,
                client,
                tx,
                amount: None,
            } => Ok(Dispute {
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
    fn test_valid_dispute() {
        let record = TransactionRecord {
            tx_type: TransactionType::Dispute,
            client: 1,
            tx: 5,
            amount: None,
        };
        let dispute = Dispute::try_from(record).unwrap();

        assert_eq!(dispute.client_id(), 1);
        assert_eq!(dispute.referenced_tx_id(), 5);
    }

    #[test]
    fn test_rejects_with_amount() {
        let record = TransactionRecord {
            tx_type: TransactionType::Dispute,
            client: 1,
            tx: 5,
            amount: Some(dec!(100)),
        };
        assert!(Dispute::try_from(record).is_err());
    }

    #[test]
    fn test_rejects_wrong_transaction_type() {
        let record = TransactionRecord {
            tx_type: TransactionType::Deposit,
            client: 1,
            tx: 5,
            amount: None,
        };
        assert!(Dispute::try_from(record).is_err());
    }
}
