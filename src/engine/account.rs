use super::Decimal;
use serde::Serialize;

pub type ClientId = u16;

/// Represents a client's account with available, held, and total balances.
#[derive(Debug, Serialize)]
pub struct Account {
    #[serde(rename = "client")]
    client_id: ClientId,
    available: Decimal,
    held: Decimal,
    total: Decimal,
    locked: bool,
}

impl Account {
    pub fn new(client_id: ClientId) -> Self {
        Self {
            client_id,
            available: Decimal::ZERO,
            held: Decimal::ZERO,
            total: Decimal::ZERO,
            locked: false,
        }
    }

    /// Returns whether the account is locked (frozen due to chargeback)
    pub fn is_locked(&self) -> bool {
        self.locked
    }

    /// Returns the available balance
    pub fn available(&self) -> Decimal {
        self.available
    }

    /// Credit the account with a deposit amount.
    /// Increases both available and total funds.
    pub fn deposit(&mut self, amount: Decimal) {
        // Sanity check
        if self.locked {
            return;
        }
        self.available += amount;
        self.total += amount;
        self.normalize();
    }

    /// Debit the account with a withdrawal amount.
    /// Caller must ensure sufficient funds and unlocked account.
    pub fn withdraw(&mut self, amount: Decimal) {
        // Sanity check
        if self.locked {
            return;
        }
        self.available -= amount;
        self.total -= amount;
        self.normalize();
    }

    /// Hold funds for a dispute.
    /// Moves funds from available to held. Total remains unchanged.
    /// Note: Available can go negative if client withdrew funds before disputing an old transaction.
    pub fn hold(&mut self, amount: Decimal) {
        // Sanity check
        if self.locked {
            return;
        }
        self.available -= amount;
        self.held += amount;
        self.normalize();
    }

    /// Release held funds (resolve a dispute).
    /// Moves funds from held back to available. Total remains unchanged.
    pub fn release(&mut self, amount: Decimal) {
        // Sanity check
        if self.locked {
            return;
        }
        self.held -= amount;
        self.available += amount;
        self.normalize();
    }

    /// Process a chargeback.
    /// Removes held funds from total and freezes the account.
    pub fn chargeback(&mut self, amount: Decimal) {
        // Sanity check
        if self.locked {
            return;
        }
        self.held -= amount;
        self.total -= amount;
        self.normalize();
        self.locked = true;
    }

    /// Normalize all decimal fields to trim trailing zeros.
    /// Keeps internal representation compact and consistent.
    fn normalize(&mut self) {
        self.available = self.available.normalize();
        self.held = self.held.normalize();
        self.total = self.total.normalize();
    }

    // Test helpers, only available in tests
    #[cfg(test)]
    pub fn held(&self) -> Decimal {
        self.held
    }

    #[cfg(test)]
    pub fn total(&self) -> Decimal {
        self.total
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use rust_decimal_macros::dec;

    #[test]
    fn test_new_account_has_zero_balances() {
        let account = Account::new(1);
        assert_eq!(account.available(), Decimal::ZERO);
        assert_eq!(account.held(), Decimal::ZERO);
        assert_eq!(account.total(), Decimal::ZERO);
        assert!(!account.is_locked());
    }

    #[test]
    fn test_deposit_increases_available_and_total() {
        let mut account = Account::new(1);
        account.deposit(dec!(100.5));

        assert_eq!(account.available(), dec!(100.5));
        assert_eq!(account.total(), dec!(100.5));
        assert_eq!(account.held(), Decimal::ZERO);
    }

    #[test]
    fn test_withdraw_decreases_available_and_total() {
        let mut account = Account::new(1);
        account.deposit(dec!(100));
        account.withdraw(dec!(40));

        assert_eq!(account.available(), dec!(60));
        assert_eq!(account.total(), dec!(60));
    }

    #[test]
    fn test_hold_moves_funds_from_available_to_held() {
        let mut account = Account::new(1);
        account.deposit(dec!(100));
        account.hold(dec!(30));

        assert_eq!(account.available(), dec!(70));
        assert_eq!(account.held(), dec!(30));
        assert_eq!(account.total(), dec!(100)); // total unchanged
    }

    #[test]
    fn test_hold_allows_negative_available() {
        let mut account = Account::new(1);
        account.deposit(dec!(50));
        account.hold(dec!(100)); // hold more than available (dispute after withdrawal)

        // Per spec: available decreases by disputed amount (can go negative)
        assert_eq!(account.available(), dec!(-50));
        assert_eq!(account.held(), dec!(100));
        assert_eq!(account.total(), dec!(50)); // total unchanged
    }

    #[test]
    fn test_release_moves_funds_from_held_to_available() {
        let mut account = Account::new(1);
        account.deposit(dec!(100));
        account.hold(dec!(30));
        account.release(dec!(30));

        assert_eq!(account.available(), dec!(100));
        assert_eq!(account.held(), Decimal::ZERO);
        assert_eq!(account.total(), dec!(100));
    }

    #[test]
    fn test_chargeback_removes_held_funds_and_locks_account() {
        let mut account = Account::new(1);
        account.deposit(dec!(100));
        account.hold(dec!(30));
        account.chargeback(dec!(30));

        assert_eq!(account.available(), dec!(70)); // unchanged from after hold
        assert_eq!(account.held(), Decimal::ZERO);
        assert_eq!(account.total(), dec!(70)); // reduced by chargeback
        assert!(account.is_locked());
    }

    #[test]
    fn test_locked_account_ignores_operations() {
        let mut account = Account::new(1);
        account.deposit(dec!(100));
        account.hold(dec!(50));
        account.chargeback(dec!(50)); // locks the account

        // All operations should be no-ops on locked account
        account.deposit(dec!(100));
        account.withdraw(dec!(10));
        account.hold(dec!(10));
        account.release(dec!(10));

        assert_eq!(account.available(), dec!(50));
        assert_eq!(account.held(), Decimal::ZERO);
        assert_eq!(account.total(), dec!(50));
    }

    #[test]
    fn test_normalize_trims_trailing_zeros() {
        let mut account = Account::new(1);
        account.deposit(dec!(100.0000));

        // After normalize, should be compact
        assert_eq!(account.available().to_string(), "100");
    }
}
