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
```
client,available,held,total,locked
1,60,0,60,false
```

## Design Decisions

### Synchronous Processing
Used **sync I/O** instead of async. For a batch CSV processor, synchronous streaming is more than sufficient and avoids the complexity of async runtimes. If this were a server handling concurrent TCP streams, we'd consider tokio.

### Only Deposits Can Be Disputed
Assumed disputes only apply to **deposit transactions**. The spec references disputing transactions that credit accounts, and withdrawals are debits. This matches typical financial dispute flows.

### Negative Available Funds Allowed
Per the spec: *"available funds should decrease by the amount disputed"*

If a client:
1. Deposits $100
2. Withdraws $80
3. The deposit is disputed

Available becomes **-$80**. This is standard in financial systems where disputes can occur after partial withdrawals.

### In-Memory Storage
Deposits and accounts are stored in `HashMap`s. Persistence (database, file-based) is out of scope. In production, you'd want:
- Transactional database for ACID guarantees
- Event sourcing for audit trails
- Distributed storage for horizontal scaling

### Soft vs Hard Errors
- **Hard errors** (stop processing): CSV parse errors, invalid transaction format
- **Soft errors** (log and continue): insufficient funds, client mismatch, already disputed

## Transaction Types

| Type | Amount | Description |
|------|--------|-------------|
| `deposit` | Required, >0 | Credit client's available and total |
| `withdrawal` | Required, >0 | Debit client's available and total (fails silently if insufficient) |
| `dispute` | Must be empty | Hold deposited funds (available → held) |
| `resolve` | Must be empty | Release held funds (held → available) |
| `chargeback` | Must be empty | Remove held funds from total and lock account |

## Validation Rules

- **Precision**: ≤4 decimal places
- **Amounts**: Must be positive (>0) for deposit/withdrawal
- **Disputes**: Can only dispute deposits that exist and belong to the same client
- **Resolve/Chargeback**: Transaction must be under active dispute

## Dependencies

| Crate | Purpose | Notes |
|-------|---------|-------|
| `rust_decimal` | Decimal arithmetic | Avoids floating-point precision issues. Well-maintained, widely used. |
| `serde` | Serialization | De-facto standard for Rust serialization |
| `csv` | CSV parsing | Handles edge cases (whitespace, quoting) |
| `thiserror` | Error types | Ergonomic error derive macros |
| `clap` | CLI parsing | Industry standard for Rust CLIs |
| `anyhow` | Error handling | Clean error context in main() |
| `log` + `env_logger` | Logging | Trace transaction processing with `RUST_LOG=trace` |

> ⚠️ **Production Note**: While `rust_decimal` is trusted and widely used, production financial systems should audit all dependencies for security. Consider using tools like `cargo-audit` and maintaining an SBOM.

## Testing

```bash
cargo test
```

**Test Coverage:**
- 32 unit tests (Account, Deposit, Withdrawal, Dispute, Resolve, Chargeback validation)
- 30 integration tests (E2E flows, edge cases, error conditions)

Key test scenarios:
- Basic deposit/withdrawal flows
- Complete dispute → resolve and dispute → chargeback flows
- Negative available after dispute on partially-withdrawn funds
- Double dispute prevention
- Client ID mismatch rejection
- Locked account behavior

## Project Structure

```
src/
├── cli/
│   ├── main.rs       # CLI entry point with error handling
│   └── commands.rs   # Clap argument definitions
├── engine/
│   ├── mod.rs        # Module exports
│   ├── payment_engine.rs  # PaymentEngine core logic
│   ├── account.rs    # Account state and balance operations
│   ├── error.rs      # Error types (hard and soft)
│   └── transaction/
│       ├── mod.rs    # Transaction enum and TryFrom
│       ├── deposit.rs
│       ├── withdrawal.rs
│       ├── dispute.rs
│       ├── resolve.rs
│       └── chargeback.rs
└── lib.rs            # Library exports

examples/
└── basic.rs          # Basic usage example

tests/
└── integration.rs    # E2E integration tests
```

## Examples

Run the basic example:

```bash
cargo run --example basic
```

Or use as a library:

```rust
use std::io::Cursor;
use payment_engine::PaymentEngine;

let transactions = "type,client,tx,amount
deposit,1,1,100.0
withdrawal,1,2,30.0";

let mut engine = PaymentEngine::new();
engine.process_transactions(Cursor::new(transactions))?;
engine.export_accounts(std::io::stdout())?;
```

## Future Improvements

If extending this system:
1. **Persistence**: Add database layer (ie Postgres with transactions)
2. **Async**: For concurrent stream processing
3. **Audit logging**: Track all state changes
4. **Rate limiting**: Prevent abuse
5. **Withdrawal disputes**: If business requires

