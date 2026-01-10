# API Testing with Postman/Newman

This directory contains API tests for the microservices using Postman collections and Newman CLI runner.

## Directory Structure

```
tests/api/
├── collections/           # Postman collection files
│   └── auth-service.postman_collection.json
├── environments/          # Environment configuration files
│   ├── dev.postman_environment.json
│   └── prod.postman_environment.json
├── reports/              # Test execution reports (generated)
└── README.md            # This file
```

## Prerequisites

### Install Newman

Newman is the CLI runner for Postman collections:

```bash
npm install -g newman newman-reporter-htmlextra
```

Or let the test script install it:

```bash
./scripts/api-test.sh --install-newman
```

### Services Must Be Running

Before running API tests, ensure the services are running:

**Development:**
```bash
./scripts/dev-up.sh
```

**Production:**
```bash
./scripts/prod-up.sh
```

## Running Tests

### Quick Start

Run all API tests in development environment:
```bash
./scripts/api-test.sh
```

### Command Options

```bash
./scripts/api-test.sh [OPTIONS]

Options:
  -e, --environment <env>   Environment to test (dev|prod, default: dev)
  -c, --collection <name>   Specific collection to run
  -v, --verbose             Show detailed Newman output
  --install-newman          Install Newman if not present
  -h, --help                Show help message
```

### Examples

**Test specific collection:**
```bash
./scripts/api-test.sh -c auth-service
```

**Test in production environment:**
```bash
./scripts/api-test.sh -e prod
```

**Verbose output:**
```bash
./scripts/api-test.sh -c auth-service -v
```

## Test Collections

### auth-service.postman_collection.json

Tests for authentication and authorization endpoints:

- **Health Check** - Verify service is running
- **User Registration** - Create new user account
- **User Login** - Authenticate and get tokens
- **Get User Profile** - Retrieve authenticated user data
- **Refresh Token** - Renew access token
- **Logout** - Invalidate tokens
- **Rate Limiting** - Verify rate limit enforcement
- **Unauthorized Access** - Verify authentication requirement

### Test Coverage

Following REST API security best practices:

- ✅ Authentication flows (login, registration, token refresh)
- ✅ Authorization (protected endpoints)
- ✅ Rate limiting verification
- ✅ Error handling (401, 429, etc.)
- ✅ Token management
- ✅ Security headers
- ✅ Input validation

## Environments

### Development (dev.postman_environment.json)

- Auth Service: `http://localhost:9005`
- Document Service: `http://localhost:9007`

### Production (prod.postman_environment.json)

- Auth Service: `http://localhost:10005`
- Document Service: `http://localhost:10007`

## Test Reports

After running tests, HTML reports are generated in `tests/api/reports/`:

- `auth-service-dev-report.html` - Auth service test results (dev)
- `auth-service-prod-report.html` - Auth service test results (prod)

Open these in a browser for detailed test execution results with:
- Request/response details
- Test assertions
- Execution timeline
- Pass/fail statistics

## Writing New Tests

### Adding Tests to Existing Collection

1. Open collection in Postman or edit JSON directly
2. Add new request under appropriate folder
3. Write test scripts in the `test` event:

```javascript
pm.test("Status code is 200", function () {
    pm.response.to.have.status(200);
});

pm.test("Response has expected field", function () {
    var jsonData = pm.response.json();
    pm.expect(jsonData).to.have.property('fieldName');
});
```

### Creating New Collection

1. Create new `.postman_collection.json` file in `collections/`
2. Follow Postman Collection v2.1.0 schema
3. Add test scripts for all requests
4. Update `api-test.sh` to include new collection

### Test Patterns

**Authentication:**
```javascript
pm.test("Response contains access token", function () {
    var jsonData = pm.response.json();
    pm.expect(jsonData).to.have.property('access_token');
    pm.environment.set("access_token", jsonData.access_token);
});
```

**Authorization:**
```javascript
pm.test("Unauthorized without token", function () {
    pm.response.to.have.status(401);
});
```

**Rate Limiting:**
```javascript
pm.test("Rate limit enforced", function () {
    pm.response.to.have.status(429);
    pm.response.to.have.header('Retry-After');
});
```

**Security Headers:**
```javascript
pm.test("Security headers present", function () {
    pm.response.to.have.header('X-Content-Type-Options');
    pm.response.to.have.header('X-Frame-Options');
});
```

## Integration with CI/CD

Add to your CI pipeline:

```bash
# Start services
./scripts/dev-up.sh

# Wait for services to be ready
sleep 10

# Run API tests
./scripts/api-test.sh -e dev

# Stop services
./scripts/dev-down.sh
```

## Best Practices

1. **Idempotent Tests** - Tests should not depend on previous runs
2. **Dynamic Data** - Use timestamps for unique test data
3. **Cleanup** - Clean up test data after runs
4. **Environment Variables** - Use environment variables for configuration
5. **Assertions** - Test both success and error cases
6. **Security** - Test authentication, authorization, and rate limiting
7. **Performance** - Monitor response times in reports

## Troubleshooting

**Services not responding:**
- Check services are running: `docker ps`
- Check logs: `docker-compose -f docker-compose.dev.yml logs -f`

**Newman not found:**
- Install: `npm install -g newman newman-reporter-htmlextra`
- Or use: `./scripts/api-test.sh --install-newman`

**Tests failing:**
- Check service health: `curl http://localhost:9005/health`
- Review HTML reports in `tests/api/reports/`
- Run with verbose: `./scripts/api-test.sh -v`

## Resources

- [Postman Documentation](https://learning.postman.com/docs/)
- [Newman Documentation](https://learning.postman.com/docs/running-collections/using-newman-cli/command-line-integration-with-newman/)
- [Writing Tests in Postman](https://learning.postman.com/docs/writing-scripts/test-scripts/)
- [Postman Collection Format](https://schema.postman.com/json/collection/v2.1.0/docs/index.html)
