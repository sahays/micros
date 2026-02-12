//! Services module for invoicing-service.

pub mod database;
mod invoice_db;
mod receipt_db;
mod tax_rate_db;

pub use database::Database;
