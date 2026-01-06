# Payment Engine

A simple toy transaction engine that processes payment transactions (deposits, withdrawals, disputes, resolves, chargebacks) and outputs final account balances.

## Usage

```bash
cargo run -- transactions.csv > accounts.csv
```

**Input**: CSV file with columns `type, client, tx, amount`  
**Output**: CSV to stdout with columns `client, available, held, total, locked`

### Example

```bash
echo "type,client,tx,amount
deposit,1,1,100.0
withdrawal,1,2,40.0
dispute,1,1,
resolve,1,1," | cargo run -- /dev/stdin
```

Output:
```csv
client,available,held,total,locked
1,60.0000,0.0000,60.0000,false
```

## Requirements Checklist

| Requirement | Status | Implementation |
|-------------|--------|----------------|
| **CLI Interface** `cargo run -- file.csv > out.csv` | ✅ | Single argument, stdout output |
| **Input Parsing** (type, client, tx, amount) | ✅ | csv + serde deserialization |
| **Precision** up to 4 decimal places | ✅ | `rust_decimal` + enforced 4dp output |
| **Whitespace handling** | ✅ | `csv::Trim::All` |
| **Deposit** credits available + total | ✅ | `Account::deposit()` |
| **Withdrawal** debits if sufficient funds | ✅ | Fails silently per spec |
| **Dispute** holds funds, decreases available | ✅ | `Account::hold()` |
| **Resolve** releases held funds | ✅ | `Account::release()` |
| **Chargeback** removes held, freezes account | ✅ | `Account::chargeback()` + locked flag |
| **Ignore invalid tx** (non-existent, wrong client) | ✅ | Soft errors, logged |
| **Streaming** (not loading all in memory) | ✅ | CSV iterator, no upfront load |
| **Client u16, Transaction u32** | ✅ | Type aliases enforced |
| **Auto-create clients** on first tx | ✅ | `HashMap::entry().or_insert()` |
| **Sample data included** | ✅ | `samples/transactions.csv` |
| **Unit + Integration tests** | ✅ | 31 unit + 30 integration tests |

### Output Precision

Per the spec: *"You can assume a precision of four places past the decimal and should output values with the same level of precision."*

Output is serialized with **exactly 4 decimal places**:
```rust
fn serialize_decimal_4dp<S: Serializer>(value: &Decimal, serializer: S) -> Result<S::Ok, S::Error> {
    serializer.serialize_str(&format!("{value:.4}"))
}
```

## Design Decisions

### Separate API Methods
`PaymentEngine` exposes two distinct methods rather than a combined `process(input, output)`:

```rust
engine.process_transactions(reader)?;  // Can be called multiple times
engine.export_accounts(writer)?;        // Called once at the end
```

**Why?** This design makes the batch semantics explicit:
- Users can process **multiple input files** before exporting final state
- Avoids confusion that transactions are processed and exported in real-time
- Export happens only when all transactions are complete (batch, not streaming output)

### Synchronous Processing
Used **sync I/O** instead of async. For a batch CSV processor, synchronous streaming is sufficient and avoids async runtime complexity. For concurrent TCP streams, we'd add tokio.

### Only Deposits Can Be Disputed
Disputes only apply to **deposit transactions**. The spec says "a dispute represents a client's claim that a transaction was erroneous" and references reversing credits. Withdrawals are debits, not credits.

### Negative Available Funds Allowed
Per the spec: *"the clients available funds should decrease by the amount disputed"*

If a client deposits $100, withdraws $80, then disputes the deposit → available becomes **-$80**. This matches real financial systems where disputes can occur after partial withdrawals.

### Soft vs Hard Errors
- **Hard errors** (stop processing): CSV parse errors, invalid transaction format
- **Soft errors** (log and continue): insufficient funds, client mismatch, already disputed, non-existent transaction

### Client Mismatch Handling
When a dispute/resolve/chargeback references a transaction belonging to a **different client**, it's treated as a **soft error** (ignored, logged). 

Rationale: The spec says to ignore invalid transactions "and assume this is an error on our partner's side." A client cannot dispute another client's deposit - that's malformed partner data, not a system failure.

### Invariant Assertions
Account operations include `debug_assert!` checks to validate:
- `total = available + held`
- Operations not called on locked accounts

These catch bugs in development with zero release overhead.

## Transaction Types

| Type | Amount | Description |
|------|--------|-------------|
| `deposit` | Required, >0 | Credit client's available and total |
| `withdrawal` | Required, >0 | Debit client's available and total (fails silently if insufficient) |
| `dispute` | Must be empty | Hold deposited funds (available → held) |
| `resolve` | Must be empty | Release held funds (held → available) |
| `chargeback` | Must be empty | Remove held funds from total and lock account |

## Testing

```bash
cargo test
```

**Test Coverage:**
- 31 unit tests (Account, Deposit, Withdrawal, Dispute, Resolve, Chargeback)
- 30 integration tests (E2E flows, edge cases, error conditions)

Key scenarios tested:
- Basic deposit/withdrawal flows
- Complete dispute → resolve and dispute → chargeback flows
- Negative available after dispute on partially-withdrawn funds
- Double dispute prevention
- Client ID mismatch rejection
- Locked account behavior
- Whitespace handling and precision validation

## Dependencies

| Crate | Purpose |
|-------|---------|
| `rust_decimal` | Decimal arithmetic (avoids floating-point issues) |
| `serde` | Serialization |
| `csv` | CSV parsing with whitespace handling |
| `thiserror` | Error type derives |
| `clap` | CLI parsing |
| `anyhow` | Error context in main() |
| `log` + `env_logger` | Logging (`RUST_LOG=debug`) |

## Project Structure

```
src/
├── cli/
│   ├── main.rs           # CLI entry point
│   └── commands.rs       # Clap argument definitions
├── engine/
│   ├── mod.rs            # Module exports
│   ├── payment_engine.rs # Core processing logic
│   ├── account.rs        # Account state + balance ops
│   ├── error.rs          # Error types
│   └── transaction/      # Transaction types + validation
└── lib.rs                # Library exports

samples/
└── transactions.csv      # Sample input data

tests/
└── integration.rs        # E2E integration tests
```

## Examples

```bash
cargo run --example basic
```

Or as a library:

```rust
use std::io::Cursor;
use payment_engine::PaymentEngine;

let csv = "type,client,tx,amount
deposit,1,1,100.0
withdrawal,1,2,30.0";

let mut engine = PaymentEngine::new();
engine.process_transactions(Cursor::new(csv))?;
engine.export_accounts(std::io::stdout())?;
```

## Note on Development

This project was developed with AI assistance (for productivity: faster test writing, boilerplate, documentation). All design decisions, architecture, and error handling strategies were guided by me. Transparency matters.

