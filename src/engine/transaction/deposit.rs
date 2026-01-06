use crate::engine::{
    error::TransactionError,
    transaction::{TransactionRecord, TransactionType},
    Decimal,
};

/// A validated deposit transaction.
///
/// Deposits credit the client's account, increasing available and total funds.
/// Tracks dispute state for dispute/resolve/chargeback flow.
#[derive(Debug, Clone)]
pub struct Deposit {
    client_id: u16,
    transaction_id: u32,
    amount: Decimal,
}

impl Deposit {
    pub fn client_id(&self) -> u16 {
        self.client_id
    }

    pub fn transaction_id(&self) -> u32 {
        self.transaction_id
    }

    pub fn amount(&self) -> Decimal {
        self.amount
    }
}

impl TryFrom<TransactionRecord> for Deposit {
    type Error = TransactionError;

    fn try_from(record: TransactionRecord) -> Result<Self, Self::Error> {
        match record {
            TransactionRecord {
                tx_type: TransactionType::Deposit,
                client,
                tx,
                amount: Some(amount),
            } if amount > Decimal::ZERO && amount.scale() <= 4 => Ok(Deposit {
                client_id: client,
                transaction_id: tx,
                amount,
            }),
            _ => Err(TransactionError::InvalidTransaction(record)),
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use rust_decimal_macros::dec;

    fn make_record(amount: Option<Decimal>) -> TransactionRecord {
        TransactionRecord {
            tx_type: TransactionType::Deposit,
            client: 1,
            tx: 1,
            amount,
        }
    }

    #[test]
    fn test_valid_deposit() {
        let record = make_record(Some(dec!(100.5)));
        let deposit = Deposit::try_from(record).unwrap();

        assert_eq!(deposit.client_id(), 1);
        assert_eq!(deposit.transaction_id(), 1);
        assert_eq!(deposit.amount(), dec!(100.5));
    }

    #[test]
    fn test_valid_deposit_with_4_decimals() {
        let record = make_record(Some(dec!(1.2345)));
        let deposit = Deposit::try_from(record).unwrap();
        assert_eq!(deposit.amount(), dec!(1.2345));
    }

    #[test]
    fn test_rejects_more_than_4_decimals() {
        let record = make_record(Some(dec!(1.23456)));
        assert!(Deposit::try_from(record).is_err());
    }

    #[test]
    fn test_rejects_negative_amount() {
        let record = make_record(Some(dec!(-100)));
        assert!(Deposit::try_from(record).is_err());
    }

    #[test]
    fn test_rejects_zero_amount() {
        let record = make_record(Some(Decimal::ZERO));
        assert!(Deposit::try_from(record).is_err());
    }

    #[test]
    fn test_rejects_missing_amount() {
        let record = make_record(None);
        assert!(Deposit::try_from(record).is_err());
    }

    #[test]
    fn test_rejects_wrong_transaction_type() {
        let record = TransactionRecord {
            tx_type: TransactionType::Withdrawal,
            client: 1,
            tx: 1,
            amount: Some(dec!(100)),
        };
        assert!(Deposit::try_from(record).is_err());
    }
}
