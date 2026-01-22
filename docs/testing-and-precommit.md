# Testing and Pre-Commit Hooks

## Pre-Commit Hook

The pre-commit hook runs automatically on `git commit` and checks only staged files.

**Install:**
```bash
ln -s ../../scripts/pre-commit.sh .git/hooks/pre-commit
```

**What it checks:**
- `cargo fmt --check` - formatting
- `cargo clippy` - linting
- `cargo test --lib` - unit tests
- `buf lint` - proto files (if buf installed)
- Integration tests (if PostgreSQL available)

**Services checked:** auth-service, service-core, document-service, genai-service, notification-service, ledger-service, payment-service

**Skip integration tests:**
```bash
SKIP_INTEG_TESTS=1 git commit -m "message"
```

## Integration Tests

Integration tests require a PostgreSQL database. The `integ-tests.sh` script handles database setup and teardown.

**Run all integration tests:**
```bash
./scripts/integ-tests.sh
```

**Run for specific service:**
```bash
./scripts/integ-tests.sh -p ledger-service
./scripts/integ-tests.sh -p auth-service
```

**Run only database tests (ignored tests):**
```bash
./scripts/integ-tests.sh -p ledger-service --ignored
```

**Skip database setup (for non-DB tests):**
```bash
./scripts/integ-tests.sh --skip-db
```

**Pass args to cargo test:**
```bash
./scripts/integ-tests.sh -- --test-threads=1
```

## Database Configuration

Integration tests use environment variables for database connection:

| Variable | Default |
|----------|---------|
| `DB_HOST` | localhost |
| `DB_PORT` | 5432 |
| `DB_USER` | postgres |
| `DB_PASSWORD` | pass@word1 |

Or create `.env.test` in the repo root with these values.

## Test Database Lifecycle

1. Creates timestamped database: `micros_test_<timestamp>`
2. Runs migrations for auth-service and ledger-service
3. Exports `TEST_DATABASE_URL` for tests
4. Drops database on exit (including Ctrl+C)

## Writing Integration Tests

Tests requiring a database should use the `#[ignore]` attribute:

```rust
#[tokio::test]
#[ignore]
async fn test_with_database() {
    let url = std::env::var("TEST_DATABASE_URL").unwrap();
    // ...
}
```

Run with `--ignored` flag or via `integ-tests.sh`.

## Unit Tests

Unit tests run without database and should not use `#[ignore]`:

```bash
cargo test -p ledger-service --lib
```

## Troubleshooting

**Pre-commit fails on formatting:**
The hook auto-runs `cargo fmt` to fix issues. Re-stage the formatted files and commit again.

**Cannot connect to PostgreSQL:**
Ensure PostgreSQL is running and credentials match. Check with:
```bash
PGPASSWORD=pass@word1 psql -h localhost -U postgres -c "SELECT 1;"
```

**Tests timeout or hang:**
Run with single thread to isolate issues:
```bash
./scripts/integ-tests.sh -- --test-threads=1
```
