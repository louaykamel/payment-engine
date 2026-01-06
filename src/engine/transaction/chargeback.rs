use crate::engine::{
    error::TransactionError,
    transaction::{TransactionRecord, TransactionType},
};

/// A validated chargeback transaction.
///
/// A chargeback is the final state of a dispute and represents the client reversing a transaction.
/// Held funds and total funds decrease by the disputed amount.
/// The client's account is immediately frozen (locked).
/// Chargebacks reference the disputed transaction by ID and do not specify an amount.
#[derive(Debug, Clone)]
pub struct Chargeback {
    client_id: u16,
    /// The ID of the transaction being charged back
    referenced_tx_id: u32,
}

impl Chargeback {
    pub fn client_id(&self) -> u16 {
        self.client_id
    }

    pub fn referenced_tx_id(&self) -> u32 {
        self.referenced_tx_id
    }
}

impl TryFrom<TransactionRecord> for Chargeback {
    type Error = TransactionError;

    fn try_from(record: TransactionRecord) -> Result<Self, Self::Error> {
        match record {
            TransactionRecord {
                tx_type: TransactionType::Chargeback,
                client,
                tx,
                amount: None,
            } => Ok(Chargeback {
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
    fn test_valid_chargeback() {
        let record = TransactionRecord {
            tx_type: TransactionType::Chargeback,
            client: 3,
            tx: 10,
            amount: None,
        };
        let chargeback = Chargeback::try_from(record).unwrap();

        assert_eq!(chargeback.client_id(), 3);
        assert_eq!(chargeback.referenced_tx_id(), 10);
    }

    #[test]
    fn test_rejects_with_amount() {
        let record = TransactionRecord {
            tx_type: TransactionType::Chargeback,
            client: 1,
            tx: 5,
            amount: Some(dec!(100)),
        };
        assert!(Chargeback::try_from(record).is_err());
    }

    #[test]
    fn test_rejects_wrong_transaction_type() {
        let record = TransactionRecord {
            tx_type: TransactionType::Resolve,
            client: 1,
            tx: 5,
            amount: None,
        };
        assert!(Chargeback::try_from(record).is_err());
    }
}
