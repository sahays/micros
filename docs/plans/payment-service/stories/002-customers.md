# Story: Customers

- [ ] **Status: Planning**
- **Epic:** [001-saas-payment-platform](../epics/001-saas-payment-platform.md)

## Summary

Implement CreateCustomer, GetCustomer, UpdateCustomer, and ListCustomers gRPC methods for managing Razorpay customer records used in subscriptions and recurring payments.

## Tasks

- [ ] Define proto messages: RazorpayCustomer
- [ ] Define proto messages: CreateCustomerRequest/Response
- [ ] Define proto messages: GetCustomerRequest/Response
- [ ] Define proto messages: UpdateCustomerRequest/Response
- [ ] Define proto messages: ListCustomersRequest/Response
- [ ] Add RazorpayCustomer MongoDB collection and indexes
- [ ] Implement Razorpay customer API client
- [ ] Implement CreateCustomer handler with Razorpay integration
- [ ] Implement GetCustomer handler
- [ ] Implement UpdateCustomer handler
- [ ] Implement ListCustomers handler with filters and pagination
- [ ] Add capability checks to all methods
- [ ] Add metering for customer operations

## gRPC Methods

### CreateCustomer
**Input:** tenant_id, user_id, name, email, contact
**Output:** customer

**Validation:**
- name is non-empty
- email is valid format
- contact is valid phone number
- user_id does not already have a Razorpay customer record for this tenant

**Business Logic:**
- Creates customer in Razorpay via API
- Stores customer record with razorpay_customer_id mapped to internal user_id

**Capability:** `payment.customer:create`

### GetCustomer
**Input:** tenant_id, customer_id (or user_id)
**Output:** customer

**Capability:** `payment.customer:read`

### UpdateCustomer
**Input:** tenant_id, customer_id, name, email, contact
**Output:** customer

**Validation:**
- Customer exists
- At least one field being updated

**Business Logic:**
- Updates customer in Razorpay via API
- Updates local record

**Capability:** `payment.customer:update`

### ListCustomers
**Input:** tenant_id, page_size, page_token
**Output:** customers[], next_page_token

**Capability:** `payment.customer:read`

## Metering

Record on each operation:
```rust
record_customer(&tenant_id, "created");
record_customer(&tenant_id, "updated");
```

## Acceptance Criteria

- [ ] CreateCustomer creates customer in Razorpay
- [ ] CreateCustomer stores razorpay_customer_id
- [ ] CreateCustomer maps user_id to razorpay_customer_id
- [ ] CreateCustomer returns AlreadyExists for duplicate user_id
- [ ] CreateCustomer validates email format
- [ ] GetCustomer returns customer by customer_id
- [ ] GetCustomer returns customer by user_id
- [ ] GetCustomer returns NOT_FOUND for missing customer
- [ ] UpdateCustomer updates details in Razorpay and locally
- [ ] ListCustomers returns only tenant's customers
- [ ] ListCustomers pagination works correctly
- [ ] All methods enforce tenant isolation
- [ ] All methods check capabilities

## Integration Tests

- [ ] Create customer with valid data returns customer
- [ ] Create customer stores razorpay_customer_id
- [ ] Create customer with duplicate user_id returns ALREADY_EXISTS
- [ ] Create customer with invalid email returns INVALID_ARGUMENT
- [ ] Get customer returns complete customer
- [ ] Get customer by user_id returns customer
- [ ] Get customer returns NOT_FOUND for missing customer
- [ ] Update customer updates name and contact
- [ ] List customers returns only tenant's customers
- [ ] List customers pagination works
- [ ] Operations without capability return PERMISSION_DENIED
