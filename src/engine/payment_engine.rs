use std::collections::{HashMap, HashSet};
use std::io::{Read, Write};

use super::account::ClientId;
use super::error::{Error, ProcessingError};
use super::transaction::{
    Chargeback, Deposit, Dispute, Resolve, Transaction, TransactionId, TransactionRecord,
    Withdrawal,
};

// Export this for testing purposes
use super::account::Account;

/// The core payment processing engine.
///
/// Processes transactions (deposits, withdrawals, disputes, resolves, chargebacks)
/// and maintains account state for all clients.
#[derive(Debug, Default)]
pub struct PaymentEngine {
    /// Maps client ID to their account state
    accounts: HashMap<ClientId, Account>,
    /// Maps transaction ID to successful deposits for dispute lookups
    deposits: HashMap<TransactionId, Deposit>,
    /// Set of disputed transactions (Under dispute)
    disputes: HashSet<TransactionId>,
}

impl PaymentEngine {
    /// Create a new `PaymentEngine` with empty accounts and transactions
    pub fn new() -> Self {
        log::trace!("PaymentEngine initialized");
        Self {
            accounts: HashMap::new(),
            deposits: HashMap::new(),
            disputes: HashSet::new(),
        }
    }

    /// Primary API: Process transactions from any source (File, `TcpStream`, etc.)
    /// Note that the CSV reader is buffered automatically, so you should not wrap rdr in a buffered reader like `io::BufReader`.
    pub fn process_transactions<R: Read>(&mut self, reader: R) -> Result<(), Error> {
        log::info!("Starting transaction processing");

        let mut csv_reader = csv::ReaderBuilder::new()
            .trim(csv::Trim::All) // trim whitespace from fields
            .from_reader(reader);

        let mut processed = 0u64;
        let mut skipped = 0u64;

        for result in csv_reader.deserialize() {
            // Step 1:Parse CSV record into raw dirty TransactionRecord
            let record: TransactionRecord = result?;

            let row_num = processed + skipped + 1;
            log::trace!(
                "[row {}] Parsing: type={:?} client={} tx={} amount={:?}",
                row_num,
                record.tx_type,
                record.client,
                record.tx,
                record.amount
            );

            // Step 2: Convert raw dirty TransactionRecord into validated Transaction
            let transaction = Transaction::try_from(record)?;

            // Step 3: Process validated Transaction
            if let Err(e) = self.process_transaction(transaction) {
                log::warn!("[row {row_num}] - Skipped: {e}");
                skipped += 1;
            } else {
                processed += 1;
            }
        }

        log::info!(
            "Processing complete: {} processed, {} skipped, {} accounts",
            processed,
            skipped,
            self.accounts.len()
        );
        Ok(())
    }

    /// Secondary API: Write final state to any sink (Stdout, File, `TcpStream`, etc.)
    /// Note that the CSV writer is buffered automatically, so you should not wrap wtr in a buffered writer like `io::BufWriter`.
    pub fn export_accounts<W: Write>(&self, writer: W) -> Result<(), Error> {
        log::info!("Exporting {} accounts", self.accounts.len());

        let mut csv_writer = csv::Writer::from_writer(writer);
        for account in self.accounts.values() {
            csv_writer.serialize(account)?;
        }
        csv_writer.flush()?;

        log::trace!("Export complete");
        Ok(())
    }

    /// Returns the number of accounts in the engine
    pub fn account_count(&self) -> usize {
        self.accounts.len()
    }

    fn process_transaction(&mut self, transaction: Transaction) -> Result<(), ProcessingError> {
        log::trace!("Processing transaction: {transaction}");
        match transaction {
            Transaction::Deposit(deposit) => self.handle_deposit(deposit),
            Transaction::Withdrawal(withdrawal) => self.handle_withdrawal(withdrawal),
            Transaction::Dispute(dispute) => self.handle_dispute(dispute),
            Transaction::Resolve(resolve) => self.handle_resolve(resolve),
            Transaction::Chargeback(chargeback) => self.handle_chargeback(chargeback),
        }
    }
}

// =============================================================================
// Transaction Handlers
// =============================================================================

impl PaymentEngine {
    fn handle_deposit(&mut self, deposit: Deposit) -> Result<(), ProcessingError> {
        log::trace!(
            "[deposit] client={} amount={}",
            deposit.client_id(),
            deposit.amount(),
        );
        let client_id = deposit.client_id();
        let amount = deposit.amount();
        let tx_id = deposit.transaction_id();

        let is_new_account = !self.accounts.contains_key(&client_id);
        let account = self
            .accounts
            .entry(client_id)
            .or_insert_with(|| Account::new(client_id));

        if is_new_account {
            log::debug!("[deposit] Created new account for client {client_id} (tx {tx_id})");
        }

        if account.is_locked() {
            return Err(ProcessingError::AccountLocked { client: client_id });
        }

        account.deposit(amount);
        self.deposits.insert(tx_id, deposit);

        log::trace!(
            "[deposit] client={} tx={} amount={} -> new_balance={}",
            client_id,
            tx_id,
            amount,
            account.available()
        );
        Ok(())
    }

    fn handle_withdrawal(&mut self, withdrawal: Withdrawal) -> Result<(), ProcessingError> {
        log::trace!(
            "[withdrawal] client={} amount={}",
            withdrawal.client_id(),
            withdrawal.amount(),
        );
        let client_id = withdrawal.client_id();
        let amount = withdrawal.amount();

        let account = self
            .accounts
            .get_mut(&client_id)
            .ok_or(ProcessingError::AccountNotFound { client: client_id })?;

        if account.is_locked() {
            return Err(ProcessingError::AccountLocked { client: client_id });
        }

        if account.available() < amount {
            return Err(ProcessingError::InsufficientFunds {
                client: client_id,
                available: account.available(),
                requested: amount,
            });
        }

        account.withdraw(amount);

        log::trace!(
            "[withdrawal] client={} amount={} -> new_balance={}",
            client_id,
            amount,
            account.available()
        );
        Ok(())
    }

    fn handle_dispute(&mut self, dispute: Dispute) -> Result<(), ProcessingError> {
        log::trace!(
            "[dispute] client={} ref_tx={}",
            dispute.client_id(),
            dispute.referenced_tx_id(),
        );
        let client_id = dispute.client_id();
        let referenced_tx_id = dispute.referenced_tx_id();

        let deposit = self.deposits.get_mut(&referenced_tx_id).ok_or(
            ProcessingError::TransactionNotFound {
                tx: referenced_tx_id,
            },
        )?;

        if deposit.client_id() != client_id {
            return Err(ProcessingError::ClientMismatch {
                tx: referenced_tx_id,
                expected: deposit.client_id(),
                got: client_id,
            });
        }

        if self.disputes.contains(&referenced_tx_id) {
            return Err(ProcessingError::AlreadyUnderDispute {
                tx: referenced_tx_id,
            });
        }

        let amount = deposit.amount();

        let account = self
            .accounts
            .get_mut(&client_id)
            .ok_or(ProcessingError::AccountNotFound { client: client_id })?;

        if account.is_locked() {
            return Err(ProcessingError::AccountLocked { client: client_id });
        }

        self.disputes.insert(referenced_tx_id);
        account.hold(amount);

        log::trace!("[dispute] client={client_id} ref_tx={referenced_tx_id} held={amount}");
        Ok(())
    }

    fn handle_resolve(&mut self, resolve: Resolve) -> Result<(), ProcessingError> {
        log::trace!(
            "[resolve] client={} ref_tx={}",
            resolve.client_id(),
            resolve.referenced_tx_id(),
        );
        let client_id = resolve.client_id();
        let referenced_tx_id = resolve.referenced_tx_id();

        let deposit = self.deposits.get_mut(&referenced_tx_id).ok_or(
            ProcessingError::TransactionNotFound {
                tx: referenced_tx_id,
            },
        )?;

        if deposit.client_id() != client_id {
            return Err(ProcessingError::ClientMismatch {
                tx: referenced_tx_id,
                expected: deposit.client_id(),
                got: client_id,
            });
        }

        if !self.disputes.contains(&referenced_tx_id) {
            return Err(ProcessingError::NotUnderDispute {
                tx: referenced_tx_id,
            });
        }

        let amount = deposit.amount();

        let account = self
            .accounts
            .get_mut(&client_id)
            .ok_or(ProcessingError::AccountNotFound { client: client_id })?;

        if account.is_locked() {
            return Err(ProcessingError::AccountLocked { client: client_id });
        }

        self.disputes.remove(&referenced_tx_id);
        account.release(amount);

        log::trace!("[resolve] client={client_id} ref_tx={referenced_tx_id} released={amount}");
        Ok(())
    }

    fn handle_chargeback(&mut self, chargeback: Chargeback) -> Result<(), ProcessingError> {
        log::trace!(
            "[chargeback] client={} ref_tx={}",
            chargeback.client_id(),
            chargeback.referenced_tx_id(),
        );
        let client_id = chargeback.client_id();
        let referenced_tx_id = chargeback.referenced_tx_id();

        let deposit = self.deposits.get_mut(&referenced_tx_id).ok_or(
            ProcessingError::TransactionNotFound {
                tx: referenced_tx_id,
            },
        )?;

        if deposit.client_id() != client_id {
            return Err(ProcessingError::ClientMismatch {
                tx: referenced_tx_id,
                expected: deposit.client_id(),
                got: client_id,
            });
        }

        if !self.disputes.contains(&referenced_tx_id) {
            return Err(ProcessingError::NotUnderDispute {
                tx: referenced_tx_id,
            });
        }

        let amount = deposit.amount();

        let account = self
            .accounts
            .get_mut(&client_id)
            .ok_or(ProcessingError::AccountNotFound { client: client_id })?;

        self.disputes.remove(&referenced_tx_id);
        account.chargeback(amount);

        log::trace!(
            "[chargeback] client={client_id} ref_tx={referenced_tx_id} amount={amount} -> account LOCKED"
        );
        Ok(())
    }
}
