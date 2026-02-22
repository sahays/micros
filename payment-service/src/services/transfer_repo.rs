use crate::models::{Transfer, TransferStatus};
use anyhow::Result;
use mongodb::bson::doc;

use super::repository::PaymentRepository;

impl PaymentRepository {
    pub async fn create_transfer(&self, transfer: Transfer) -> Result<()> {
        self.transfer_collection.insert_one(transfer, None).await?;
        Ok(())
    }

    pub async fn get_transfer_in_tenant(
        &self,
        app_id: &str,
        tenant_id: &str,
        id: &str,
    ) -> Result<Option<Transfer>> {
        let filter = doc! {
            "_id": id,
            "app_id": app_id,
            "tenant_id": tenant_id
        };
        Ok(self.transfer_collection.find_one(filter, None).await?)
    }

    pub async fn get_transfer_by_razorpay_id(
        &self,
        razorpay_transfer_id: &str,
    ) -> Result<Option<Transfer>> {
        let filter = doc! { "razorpay_transfer_id": razorpay_transfer_id };
        Ok(self.transfer_collection.find_one(filter, None).await?)
    }

    pub async fn update_transfer_status_by_razorpay_id(
        &self,
        razorpay_transfer_id: &str,
        status: TransferStatus,
    ) -> Result<()> {
        let filter = doc! { "razorpay_transfer_id": razorpay_transfer_id };
        let update = doc! {
            "$set": {
                "status": mongodb::bson::to_bson(&status)?,
                "updated_at": mongodb::bson::DateTime::now()
            }
        };
        self.transfer_collection
            .update_one(filter, update, None)
            .await?;
        Ok(())
    }

    pub async fn update_transfer_in_tenant(
        &self,
        app_id: &str,
        tenant_id: &str,
        id: &str,
        update: mongodb::bson::Document,
    ) -> Result<()> {
        let filter = doc! {
            "_id": id,
            "app_id": app_id,
            "tenant_id": tenant_id
        };
        self.transfer_collection
            .update_one(filter, doc! { "$set": update }, None)
            .await?;
        Ok(())
    }

    #[allow(clippy::too_many_arguments)]
    pub async fn list_transfers_in_tenant(
        &self,
        app_id: &str,
        tenant_id: &str,
        linked_account_id: Option<&str>,
        payment_id: Option<&str>,
        status_filter: Option<TransferStatus>,
        limit: i64,
        offset: u64,
    ) -> Result<(Vec<Transfer>, i64)> {
        use futures::TryStreamExt;
        use mongodb::options::FindOptions;

        let mut filter = doc! {
            "app_id": app_id,
            "tenant_id": tenant_id
        };

        if let Some(la_id) = linked_account_id {
            filter.insert("linked_account_id", la_id);
        }
        if let Some(p_id) = payment_id {
            filter.insert("payment_id", p_id);
        }
        if let Some(status) = status_filter {
            filter.insert("status", mongodb::bson::to_bson(&status)?);
        }

        let total_count = self
            .transfer_collection
            .count_documents(filter.clone(), None)
            .await? as i64;

        let options = FindOptions::builder()
            .sort(doc! { "created_at": -1 })
            .skip(offset)
            .limit(limit)
            .build();

        let cursor = self.transfer_collection.find(filter, Some(options)).await?;

        let transfers: Vec<Transfer> = cursor.try_collect().await?;
        Ok((transfers, total_count))
    }
}
