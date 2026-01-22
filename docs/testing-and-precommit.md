# Testing and Pre-Commit Hooks

## Pre-Commit Hook

The pre-commit hook ensures every commit is properly formatted, lint-free, and fully tested across ALL services.

**Install:**
```bash
ln -s ../../scripts/pre-commit.sh .git/hooks/pre-commit
```

**What it runs:**
1. `cargo fmt --check` - formatting (changed services only)
2. `cargo clippy -D warnings -D clippy::all` - strict linting (changed services only)
3. `cargo test --lib` - unit tests (changed services only)
4. `buf lint` - proto files (if changed)
5. **Integration tests for ALL services** (PostgreSQL + MongoDB required)

**Skip integration tests (use sparingly):**
```bash
SKIP_INTEG_TESTS=1 git commit -m "message"
```

## Database Requirements

Both databases must be running for commits:
- **PostgreSQL**: localhost:5432 (auth-service, ledger-service)
- **MongoDB**: localhost:27017 (document-service, genai-service, notification-service)

## Integration Tests

The `integ-tests.sh` script handles database setup, migrations, and test execution.

**Run all integration tests:**
```bash
./scripts/integ-tests.sh
```

**Run for specific service:**
```bash
./scripts/integ-tests.sh -p ledger-service
./scripts/integ-tests.sh -p document-service
```

**Pass args to cargo test:**
```bash
./scripts/integ-tests.sh -- --test-threads=1
```

## Service Database Mapping

| Service | Database | Test Pattern |
|---------|----------|--------------|
| auth-service | PostgreSQL | `#[ignore]` tests |
| ledger-service | PostgreSQL | `#[ignore]` tests |
| document-service | MongoDB | Regular tests |
| genai-service | MongoDB | Regular tests |
| notification-service | MongoDB | Regular tests |

## PostgreSQL Test Lifecycle

1. Creates timestamped database: `micros_test_<timestamp>`
2. Runs migrations for auth-service and ledger-service
3. Exports `TEST_DATABASE_URL` for tests
4. Runs tests with `--ignored --test-threads=1` (sequential to prevent race conditions)
5. Drops database on exit (including Ctrl+C)

## MongoDB Test Lifecycle

1. Each test creates its own database (e.g., `document_test_<uuid>`)
2. Tests clean up their own databases
3. No global setup/teardown needed

## Environment Variables

| Variable | Default | Description |
|----------|---------|-------------|
| `DB_HOST` | localhost | PostgreSQL host |
| `DB_PORT` | 5432 | PostgreSQL port |
| `DB_USER` | postgres | PostgreSQL user |
| `DB_PASSWORD` | pass@word1 | PostgreSQL password |
| `MONGODB_URI` | mongodb://localhost:27017 | MongoDB connection |

## Troubleshooting

**Pre-commit fails on formatting:**
The hook auto-runs `cargo fmt`. Re-stage files and commit again.

**PostgreSQL not available:**
```bash
PGPASSWORD=pass@word1 psql -h localhost -U postgres -c "SELECT 1;"
```

**MongoDB not available:**
```bash
mongosh --eval "db.runCommand({ping:1})"
```

**Tests failing with race conditions:**
PostgreSQL tests already run with `--test-threads=1`. For MongoDB tests:
```bash
./scripts/integ-tests.sh -p document-service -- --test-threads=1
```
