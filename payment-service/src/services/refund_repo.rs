use crate::models::{Refund, RefundStatus};
use anyhow::Result;
use mongodb::bson::doc;

use super::repository::PaymentRepository;

impl PaymentRepository {
    pub async fn create_refund(&self, refund: Refund) -> Result<()> {
        self.refund_collection.insert_one(refund, None).await?;
        Ok(())
    }

    pub async fn get_refund_in_tenant(
        &self,
        app_id: &str,
        tenant_id: &str,
        id: &str,
    ) -> Result<Option<Refund>> {
        let filter = doc! {
            "_id": id,
            "app_id": app_id,
            "tenant_id": tenant_id
        };
        Ok(self.refund_collection.find_one(filter, None).await?)
    }

    pub async fn get_refund_by_razorpay_id(
        &self,
        razorpay_refund_id: &str,
    ) -> Result<Option<Refund>> {
        let filter = doc! { "razorpay_refund_id": razorpay_refund_id };
        Ok(self.refund_collection.find_one(filter, None).await?)
    }

    pub async fn update_refund_status_by_razorpay_id(
        &self,
        razorpay_refund_id: &str,
        status: RefundStatus,
    ) -> Result<()> {
        let filter = doc! { "razorpay_refund_id": razorpay_refund_id };
        let update = doc! {
            "$set": {
                "status": mongodb::bson::to_bson(&status)?,
                "updated_at": mongodb::bson::DateTime::now()
            }
        };
        self.refund_collection
            .update_one(filter, update, None)
            .await?;
        Ok(())
    }

    pub async fn list_refunds_in_tenant(
        &self,
        app_id: &str,
        tenant_id: &str,
        payment_id: Option<&str>,
        status_filter: Option<RefundStatus>,
        limit: i64,
        offset: u64,
    ) -> Result<(Vec<Refund>, i64)> {
        use futures::TryStreamExt;
        use mongodb::options::FindOptions;

        let mut filter = doc! {
            "app_id": app_id,
            "tenant_id": tenant_id
        };

        if let Some(pid) = payment_id {
            filter.insert("payment_id", pid);
        }
        if let Some(status) = status_filter {
            filter.insert("status", mongodb::bson::to_bson(&status)?);
        }

        let total_count = self
            .refund_collection
            .count_documents(filter.clone(), None)
            .await? as i64;

        let options = FindOptions::builder()
            .sort(doc! { "created_at": -1 })
            .skip(offset)
            .limit(limit)
            .build();

        let cursor = self.refund_collection.find(filter, Some(options)).await?;

        let refunds: Vec<Refund> = cursor.try_collect().await?;
        Ok((refunds, total_count))
    }

    /// Sum of processed refund amounts for a given payment.
    pub async fn sum_refunds_for_payment(
        &self,
        app_id: &str,
        tenant_id: &str,
        payment_id: &str,
    ) -> Result<u64> {
        use futures::TryStreamExt;

        let filter = doc! {
            "app_id": app_id,
            "tenant_id": tenant_id,
            "payment_id": payment_id,
            "status": { "$ne": "FAILED" }
        };

        let cursor = self.refund_collection.find(filter, None).await?;
        let refunds: Vec<Refund> = cursor.try_collect().await?;

        Ok(refunds.iter().map(|r| r.amount).sum())
    }
}
