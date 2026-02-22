use crate::models::{
    LinkedAccount, PaymentLink, PaymentMethod, RazorpayCustomer, RazorpayPlan,
    RazorpaySubscription, Refund, Settlement, Transaction, TransactionStatus, Transfer,
};
use anyhow::Result;
use mongodb::options::IndexOptions;
use mongodb::{bson::doc, Collection, Database, IndexModel};

#[derive(Clone)]
pub struct PaymentRepository {
    pub(crate) transaction_collection: Collection<Transaction>,
    payment_method_collection: Collection<PaymentMethod>,
    pub(crate) linked_account_collection: Collection<LinkedAccount>,
    pub(crate) customer_collection: Collection<RazorpayCustomer>,
    pub(crate) transfer_collection: Collection<Transfer>,
    pub(crate) settlement_collection: Collection<Settlement>,
    pub(crate) plan_collection: Collection<RazorpayPlan>,
    pub(crate) subscription_collection: Collection<RazorpaySubscription>,
    pub(crate) payment_link_collection: Collection<PaymentLink>,
    pub(crate) refund_collection: Collection<Refund>,
}

impl PaymentRepository {
    pub fn new(db: &Database) -> Self {
        Self {
            transaction_collection: db.collection("transactions"),
            payment_method_collection: db.collection("payment_methods"),
            linked_account_collection: db.collection("linked_accounts"),
            customer_collection: db.collection("customers"),
            transfer_collection: db.collection("transfers"),
            settlement_collection: db.collection("settlements"),
            plan_collection: db.collection("plans"),
            subscription_collection: db.collection("subscriptions"),
            payment_link_collection: db.collection("payment_links"),
            refund_collection: db.collection("refunds"),
        }
    }

    /// Initialize database indexes for tenant-scoped queries.
    pub async fn init_indexes(&self) -> Result<()> {
        // Transaction indexes
        let tenant_tx_index = IndexModel::builder()
            .keys(doc! { "app_id": 1, "tenant_id": 1, "_id": 1 })
            .options(
                IndexOptions::builder()
                    .name("tenant_transaction_idx".to_string())
                    .build(),
            )
            .build();

        let user_tx_index = IndexModel::builder()
            .keys(doc! { "app_id": 1, "tenant_id": 1, "user_id": 1 })
            .options(
                IndexOptions::builder()
                    .name("tenant_user_transaction_idx".to_string())
                    .build(),
            )
            .build();

        let status_tx_index = IndexModel::builder()
            .keys(doc! { "app_id": 1, "tenant_id": 1, "status": 1 })
            .options(
                IndexOptions::builder()
                    .name("tenant_status_transaction_idx".to_string())
                    .build(),
            )
            .build();

        let external_ref_tx_index = IndexModel::builder()
            .keys(doc! { "app_id": 1, "tenant_id": 1, "external_reference": 1 })
            .options(
                IndexOptions::builder()
                    .name("tenant_external_ref_transaction_idx".to_string())
                    .sparse(true)
                    .build(),
            )
            .build();

        self.transaction_collection
            .create_indexes(
                [
                    tenant_tx_index,
                    user_tx_index,
                    status_tx_index,
                    external_ref_tx_index,
                ],
                None,
            )
            .await?;

        // Payment method indexes
        let tenant_pm_index = IndexModel::builder()
            .keys(doc! { "app_id": 1, "tenant_id": 1 })
            .options(
                IndexOptions::builder()
                    .name("tenant_payment_method_idx".to_string())
                    .build(),
            )
            .build();

        self.payment_method_collection
            .create_indexes([tenant_pm_index], None)
            .await?;

        // Linked account indexes
        let tenant_la_index = IndexModel::builder()
            .keys(doc! { "app_id": 1, "tenant_id": 1 })
            .options(
                IndexOptions::builder()
                    .name("tenant_linked_account_idx".to_string())
                    .build(),
            )
            .build();

        let razorpay_la_index = IndexModel::builder()
            .keys(doc! { "razorpay_account_id": 1 })
            .options(
                IndexOptions::builder()
                    .name("razorpay_account_id_idx".to_string())
                    .unique(true)
                    .build(),
            )
            .build();

        self.linked_account_collection
            .create_indexes([tenant_la_index, razorpay_la_index], None)
            .await?;

        // Customer indexes
        let tenant_customer_index = IndexModel::builder()
            .keys(doc! { "app_id": 1, "tenant_id": 1, "user_id": 1 })
            .options(
                IndexOptions::builder()
                    .name("tenant_user_customer_idx".to_string())
                    .unique(true)
                    .build(),
            )
            .build();

        self.customer_collection
            .create_indexes([tenant_customer_index], None)
            .await?;

        // Transfer indexes
        let tenant_transfer_index = IndexModel::builder()
            .keys(doc! { "app_id": 1, "tenant_id": 1, "status": 1 })
            .options(
                IndexOptions::builder()
                    .name("tenant_status_transfer_idx".to_string())
                    .build(),
            )
            .build();

        let razorpay_transfer_index = IndexModel::builder()
            .keys(doc! { "razorpay_transfer_id": 1 })
            .options(
                IndexOptions::builder()
                    .name("razorpay_transfer_id_idx".to_string())
                    .unique(true)
                    .build(),
            )
            .build();

        self.transfer_collection
            .create_indexes([tenant_transfer_index, razorpay_transfer_index], None)
            .await?;

        // Settlement indexes
        let tenant_settlement_index = IndexModel::builder()
            .keys(doc! { "app_id": 1, "tenant_id": 1, "status": 1 })
            .options(
                IndexOptions::builder()
                    .name("tenant_status_settlement_idx".to_string())
                    .build(),
            )
            .build();

        let razorpay_settlement_index = IndexModel::builder()
            .keys(doc! { "razorpay_settlement_id": 1 })
            .options(
                IndexOptions::builder()
                    .name("razorpay_settlement_id_idx".to_string())
                    .unique(true)
                    .build(),
            )
            .build();

        self.settlement_collection
            .create_indexes([tenant_settlement_index, razorpay_settlement_index], None)
            .await?;

        // Plan indexes
        let tenant_plan_index = IndexModel::builder()
            .keys(doc! { "app_id": 1, "tenant_id": 1 })
            .options(
                IndexOptions::builder()
                    .name("tenant_plan_idx".to_string())
                    .build(),
            )
            .build();

        self.plan_collection
            .create_indexes([tenant_plan_index], None)
            .await?;

        // Subscription indexes
        let tenant_sub_index = IndexModel::builder()
            .keys(doc! { "app_id": 1, "tenant_id": 1, "status": 1 })
            .options(
                IndexOptions::builder()
                    .name("tenant_status_subscription_idx".to_string())
                    .build(),
            )
            .build();

        let razorpay_sub_index = IndexModel::builder()
            .keys(doc! { "razorpay_subscription_id": 1 })
            .options(
                IndexOptions::builder()
                    .name("razorpay_subscription_id_idx".to_string())
                    .unique(true)
                    .build(),
            )
            .build();

        self.subscription_collection
            .create_indexes([tenant_sub_index, razorpay_sub_index], None)
            .await?;

        // Payment link indexes
        let tenant_pl_index = IndexModel::builder()
            .keys(doc! { "app_id": 1, "tenant_id": 1, "status": 1 })
            .options(
                IndexOptions::builder()
                    .name("tenant_status_payment_link_idx".to_string())
                    .build(),
            )
            .build();

        let razorpay_pl_index = IndexModel::builder()
            .keys(doc! { "razorpay_payment_link_id": 1 })
            .options(
                IndexOptions::builder()
                    .name("razorpay_payment_link_id_idx".to_string())
                    .unique(true)
                    .build(),
            )
            .build();

        self.payment_link_collection
            .create_indexes([tenant_pl_index, razorpay_pl_index], None)
            .await?;

        // Refund indexes
        let tenant_refund_index = IndexModel::builder()
            .keys(doc! { "app_id": 1, "tenant_id": 1, "payment_id": 1 })
            .options(
                IndexOptions::builder()
                    .name("tenant_payment_refund_idx".to_string())
                    .build(),
            )
            .build();

        let razorpay_refund_index = IndexModel::builder()
            .keys(doc! { "razorpay_refund_id": 1 })
            .options(
                IndexOptions::builder()
                    .name("razorpay_refund_id_idx".to_string())
                    .unique(true)
                    .build(),
            )
            .build();

        self.refund_collection
            .create_indexes([tenant_refund_index, razorpay_refund_index], None)
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

    /// Get a transaction by ID within a specific tenant (app_id, tenant_id).
    pub async fn get_transaction_in_tenant(
        &self,
        app_id: &str,
        tenant_id: &str,
        id: &str,
    ) -> Result<Option<Transaction>> {
        let filter = doc! {
            "_id": id,
            "app_id": app_id,
            "tenant_id": tenant_id
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
        tenant_id: &str,
        id: &str,
        status: TransactionStatus,
    ) -> Result<()> {
        let filter = doc! {
            "_id": id,
            "app_id": app_id,
            "tenant_id": tenant_id
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
        tenant_id: &str,
        status_filter: Option<TransactionStatus>,
        limit: i64,
        offset: u64,
    ) -> Result<(Vec<Transaction>, i64)> {
        use futures::TryStreamExt;
        use mongodb::options::FindOptions;

        let mut filter = doc! {
            "app_id": app_id,
            "tenant_id": tenant_id
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

    /// Get a transaction by external reference (UTR, cheque number, etc.) within a tenant.
    pub async fn get_transaction_by_external_ref(
        &self,
        app_id: &str,
        tenant_id: &str,
        external_reference: &str,
    ) -> Result<Option<Transaction>> {
        let filter = doc! {
            "app_id": app_id,
            "tenant_id": tenant_id,
            "external_reference": external_reference
        };
        let transaction = self.transaction_collection.find_one(filter, None).await?;
        Ok(transaction)
    }
}
