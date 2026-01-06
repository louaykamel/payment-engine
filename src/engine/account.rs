use super::Decimal;
use serde::{Deserialize, Serialize, Serializer};

pub type ClientId = u16;

/// Serialize Decimal with exactly 4 decimal places
fn serialize_decimal_4dp<S: Serializer>(value: &Decimal, serializer: S) -> Result<S::Ok, S::Error> {
    serializer.serialize_str(&format!("{value:.4}"))
}

/// Represents a client's account with available, held, and total balances.
#[derive(Debug, Serialize, Deserialize, PartialEq)]
pub struct Account {
    #[serde(rename = "client")]
    client_id: ClientId,
    #[serde(serialize_with = "serialize_decimal_4dp")]
    available: Decimal,
    #[serde(serialize_with = "serialize_decimal_4dp")]
    held: Decimal,
    #[serde(serialize_with = "serialize_decimal_4dp")]
    total: Decimal,
    locked: bool,
}

impl Account {
    pub(super) fn new(client_id: ClientId) -> Self {
        Self {
            client_id,
            available: Decimal::ZERO,
            held: Decimal::ZERO,
            total: Decimal::ZERO,
            locked: false,
        }
    }

    /// Returns the client ID
    pub fn client_id(&self) -> ClientId {
        self.client_id
    }

    /// Returns the available balance
    pub fn available(&self) -> Decimal {
        self.available
    }

    /// Returns the held balance
    pub fn held(&self) -> Decimal {
        self.held
    }

    /// Returns the total balance
    pub fn total(&self) -> Decimal {
        self.total
    }

    /// Returns whether the account is locked (frozen due to chargeback)
    pub fn is_locked(&self) -> bool {
        self.locked
    }

    /// Credit the account with a deposit amount.
    /// Increases both available and total funds.
    ///
    /// # Panics (debug only)
    /// Panics if called on a locked account.
    pub(super) fn deposit(&mut self, amount: Decimal) {
        debug_assert!(!self.locked, "deposit called on locked account");
        self.available += amount;
        self.total += amount;
        self.normalize();
        #[cfg(debug_assertions)]
        self.assert_invariant();
    }

    /// Debit the account with a withdrawal amount.
    /// Caller must ensure sufficient funds and unlocked account.
    ///
    /// # Panics (debug only)
    /// Panics if called on a locked account.
    pub(super) fn withdraw(&mut self, amount: Decimal) {
        debug_assert!(!self.locked, "withdraw called on locked account");
        self.available -= amount;
        self.total -= amount;
        self.normalize();
        #[cfg(debug_assertions)]
        self.assert_invariant();
    }

    /// Hold funds for a dispute.
    /// Moves funds from available to held. Total remains unchanged.
    /// Note: Available can go negative if client withdrew funds before disputing an old transaction.
    ///
    /// # Panics (debug only)
    /// Panics if called on a locked account.
    pub(super) fn hold(&mut self, amount: Decimal) {
        debug_assert!(!self.locked, "hold called on locked account");
        self.available -= amount;
        self.held += amount;
        self.normalize();
        #[cfg(debug_assertions)]
        self.assert_invariant();
    }

    /// Release held funds (resolve a dispute).
    /// Moves funds from held back to available. Total remains unchanged.
    ///
    /// # Panics (debug only)
    /// Panics if called on a locked account.
    pub(super) fn release(&mut self, amount: Decimal) {
        debug_assert!(!self.locked, "release called on locked account");
        self.held -= amount;
        self.available += amount;
        self.normalize();
        #[cfg(debug_assertions)]
        self.assert_invariant();
    }

    /// Process a chargeback.
    /// Removes held funds from total and freezes the account.
    ///
    /// # Panics (debug only)
    /// Panics if called on a locked account.
    pub(super) fn chargeback(&mut self, amount: Decimal) {
        debug_assert!(!self.locked, "chargeback called on locked account");
        self.held -= amount;
        self.total -= amount;
        self.normalize();
        self.locked = true;
        #[cfg(debug_assertions)]
        self.assert_invariant();
    }

    /// Assert the fundamental accounting invariant:
    /// total = available + held (This should )
    /// available = total - held
    /// held = total - available
    #[cfg(debug_assertions)]
    fn assert_invariant(&self) {
        debug_assert_eq!(
            self.total,
            self.available + self.held,
            "Invariant violated: total ({}) != available ({}) + held ({})",
            self.total,
            self.available,
            self.held
        );
        // Anything below this line should not panic because the previous check should catch it
        debug_assert_eq!(
            self.available,
            self.total - self.held,
            "Invariant violated: available ({}) != total ({}) - held ({})",
            self.available,
            self.total,
            self.held
        );
        debug_assert_eq!(
            self.held,
            self.total - self.available,
            "Invariant violated: held ({}) != total ({}) - available ({})",
            self.held,
            self.total,
            self.available
        );
    }

    /// Normalize all decimal fields to trim trailing zeros.
    /// Keeps internal representation compact and consistent.
    /// NOTE:
    fn normalize(&mut self) {
        self.available = self.available.normalize();
        self.held = self.held.normalize();
        self.total = self.total.normalize();
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
    fn test_normalize_trims_trailing_zeros() {
        let mut account = Account::new(1);
        account.deposit(dec!(100.0000));

        // After normalize, should be compact
        assert_eq!(account.available().to_string(), "100");
    }
}
