mod common;

#[path = "payment_tests/customer.rs"]
mod customer;

#[path = "payment_tests/direct_payments.rs"]
mod direct_payments;

#[path = "payment_tests/refund.rs"]
mod refund;

#[path = "payment_tests/linked_account.rs"]
mod linked_account;

#[path = "payment_tests/payment_link.rs"]
mod payment_link;

#[path = "payment_tests/settlement.rs"]
mod settlement;

#[path = "payment_tests/subscription.rs"]
mod subscription;

#[path = "payment_tests/transfer.rs"]
mod transfer;
