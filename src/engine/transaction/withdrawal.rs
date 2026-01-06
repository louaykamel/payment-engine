use crate::engine::{
    error::TransactionError,
    transaction::{TransactionRecord, TransactionType},
    Decimal,
};

/// A validated withdrawal transaction.
///
/// Withdrawals debit the client's account, decreasing available and total funds.
/// If the client does not have sufficient available funds, the withdrawal should fail.
#[derive(Debug, Clone)]
pub struct Withdrawal {
    client_id: u16,
    transaction_id: u32,
    amount: Decimal,
}

impl Withdrawal {
    pub fn client_id(&self) -> u16 {
        self.client_id
    }

    #[allow(unused)]
    pub fn transaction_id(&self) -> u32 {
        self.transaction_id
    }

    pub fn amount(&self) -> Decimal {
        self.amount
    }
}

impl TryFrom<TransactionRecord> for Withdrawal {
    type Error = TransactionError;

    fn try_from(record: TransactionRecord) -> Result<Self, Self::Error> {
        match record {
            TransactionRecord {
                tx_type: TransactionType::Withdrawal,
                client,
                tx,
                amount: Some(amount),
            } if amount > Decimal::ZERO && amount.scale() <= 4 => Ok(Withdrawal {
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
            tx_type: TransactionType::Withdrawal,
            client: 1,
            tx: 1,
            amount,
        }
    }

    #[test]
    fn test_valid_withdrawal() {
        let record = make_record(Some(dec!(50.25)));
        let withdrawal = Withdrawal::try_from(record).unwrap();

        assert_eq!(withdrawal.client_id(), 1);
        assert_eq!(withdrawal.transaction_id(), 1);
        assert_eq!(withdrawal.amount(), dec!(50.25));
    }

    #[test]
    fn test_valid_withdrawal_with_4_decimals() {
        let record = make_record(Some(dec!(1.2345)));
        let withdrawal = Withdrawal::try_from(record).unwrap();
        assert_eq!(withdrawal.amount(), dec!(1.2345));
    }

    #[test]
    fn test_rejects_more_than_4_decimals() {
        let record = make_record(Some(dec!(1.23456)));
        assert!(Withdrawal::try_from(record).is_err());
    }

    #[test]
    fn test_rejects_negative_amount() {
        let record = make_record(Some(dec!(-100)));
        assert!(Withdrawal::try_from(record).is_err());
    }

    #[test]
    fn test_rejects_zero_amount() {
        let record = make_record(Some(Decimal::ZERO));
        assert!(Withdrawal::try_from(record).is_err());
    }

    #[test]
    fn test_rejects_missing_amount() {
        let record = make_record(None);
        assert!(Withdrawal::try_from(record).is_err());
    }

    #[test]
    fn test_rejects_wrong_transaction_type() {
        let record = TransactionRecord {
            tx_type: TransactionType::Deposit,
            client: 1,
            tx: 1,
            amount: Some(dec!(100)),
        };
        assert!(Withdrawal::try_from(record).is_err());
    }
}
