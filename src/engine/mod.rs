//! Payment engine module.
//!
//! This module contains the core payment processing logic including:
//! - `PaymentEngine` - The main transaction processor
//! - `Account` - Client account state management
//! - `Transaction` types - Deposit, Withdrawal, Dispute, Resolve, Chargeback
//! - `Error` types - Processing and validation errors

mod account;
mod error;
mod payment_engine;
mod transaction;

pub(crate) use rust_decimal::Decimal;

pub use payment_engine::PaymentEngine;
