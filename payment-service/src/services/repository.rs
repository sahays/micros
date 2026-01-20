use crate::models::{PaymentMethod, Transaction, TransactionStatus};
use anyhow::Result;
use mongodb::options::IndexOptions;
use mongodb::{
    bson::doc,
    Collection, Database, IndexModel,
};

#[derive(Clone)]
pub struct PaymentRepository {
    transaction_collection: Collection<Transaction>,
    payment_method_collection: Collection<PaymentMethod>,
}

impl PaymentRepository {
    pub fn new(db: &Database) -> Self {
        Self {
            transaction_collection: db.collection("transactions"),
            payment_method_collection: db.collection("payment_methods"),
        }
    }

    /// Initialize database indexes for tenant-scoped queries.
    pub async fn init_indexes(&self) -> Result<()> {
        // Compound index on (app_id, org_id, _id) for tenant-scoped transaction lookups
        let tenant_tx_index = IndexModel::builder()
            .keys(doc! { "app_id": 1, "org_id": 1, "_id": 1 })
            .options(
                IndexOptions::builder()
                    .name("tenant_transaction_idx".to_string())
                    .build(),
            )
            .build();

        // Compound index on (app_id, org_id, user_id) for user-scoped queries
        let user_tx_index = IndexModel::builder()
            .keys(doc! { "app_id": 1, "org_id": 1, "user_id": 1 })
            .options(
                IndexOptions::builder()
                    .name("tenant_user_transaction_idx".to_string())
                    .build(),
            )
            .build();

        // Compound index on (app_id, org_id, status) for tenant-scoped status queries
        let status_tx_index = IndexModel::builder()
            .keys(doc! { "app_id": 1, "org_id": 1, "status": 1 })
            .options(
                IndexOptions::builder()
                    .name("tenant_status_transaction_idx".to_string())
                    .build(),
            )
            .build();

        self.transaction_collection
            .create_indexes([tenant_tx_index, user_tx_index, status_tx_index], None)
            .await?;

        // Compound index on (app_id, org_id) for tenant-scoped payment method queries
        let tenant_pm_index = IndexModel::builder()
            .keys(doc! { "app_id": 1, "org_id": 1 })
            .options(
                IndexOptions::builder()
                    .name("tenant_payment_method_idx".to_string())
                    .build(),
            )
            .build();

        self.payment_method_collection
            .create_indexes([tenant_pm_index], None)
            .await?;

        tracing::info!("Payment service indexes initialized");
        Ok(())
    }

    pub async fn create_transaction(&self, transaction: Transaction) -> Result<()> {
        self.transaction_collection
            .insert_one(transaction, None)
            .await?;
        Ok(())
    }

    pub async fn get_transaction(&self, id: &str) -> Result<Option<Transaction>> {
        let filter = doc! { "_id": id };
        let transaction = self.transaction_collection.find_one(filter, None).await?;
        Ok(transaction)
    }

    /// Get a transaction by ID within a specific tenant (app_id, org_id).
    pub async fn get_transaction_in_tenant(
        &self,
        app_id: &str,
        org_id: &str,
        id: &str,
    ) -> Result<Option<Transaction>> {
        let filter = doc! {
            "_id": id,
            "app_id": app_id,
            "org_id": org_id
        };
        let transaction = self.transaction_collection.find_one(filter, None).await?;
        Ok(transaction)
    }

    pub async fn update_transaction_status(
        &self,
        id: &str,
        status: TransactionStatus,
    ) -> Result<()> {
        let filter = doc! { "_id": id };
        let update = doc! {
            "$set": {
                "status": mongodb::bson::to_bson(&status)?,
                "updated_at": mongodb::bson::DateTime::now()
            }
        };
        self.transaction_collection
            .update_one(filter, update, None)
            .await?;
        Ok(())
    }

    /// Update transaction status within a specific tenant.
    pub async fn update_transaction_status_in_tenant(
        &self,
        app_id: &str,
        org_id: &str,
        id: &str,
        status: TransactionStatus,
    ) -> Result<()> {
        let filter = doc! {
            "_id": id,
            "app_id": app_id,
            "org_id": org_id
        };
        let update = doc! {
            "$set": {
                "status": mongodb::bson::to_bson(&status)?,
                "updated_at": mongodb::bson::DateTime::now()
            }
        };
        self.transaction_collection
            .update_one(filter, update, None)
            .await?;
        Ok(())
    }

    pub async fn save_payment_method(&self, method: PaymentMethod) -> Result<()> {
        self.payment_method_collection
            .insert_one(method, None)
            .await?;
        Ok(())
    }

    /// List transactions within a specific tenant with optional status filter.
    pub async fn list_transactions_in_tenant(
        &self,
        app_id: &str,
        org_id: &str,
        status_filter: Option<TransactionStatus>,
        limit: i64,
        offset: u64,
    ) -> Result<(Vec<Transaction>, i64)> {
        use futures::TryStreamExt;
        use mongodb::options::FindOptions;

        let mut filter = doc! {
            "app_id": app_id,
            "org_id": org_id
        };

        if let Some(status) = status_filter {
            filter.insert("status", mongodb::bson::to_bson(&status)?);
        }

        // Get total count
        let total_count = self
            .transaction_collection
            .count_documents(filter.clone(), None)
            .await? as i64;

        // Get paginated results
        let options = FindOptions::builder()
            .sort(doc! { "created_at": -1 })
            .skip(offset)
            .limit(limit)
            .build();

        let cursor = self
            .transaction_collection
            .find(filter, Some(options))
            .await?;

        let transactions: Vec<Transaction> = cursor.try_collect().await?;

        Ok((transactions, total_count))
    }
}
