use crate::models::{PaymentLink, PaymentLinkStatus};
use anyhow::Result;
use mongodb::bson::doc;

use super::repository::PaymentRepository;

impl PaymentRepository {
    pub async fn create_payment_link(&self, link: PaymentLink) -> Result<()> {
        self.payment_link_collection.insert_one(link, None).await?;
        Ok(())
    }

    pub async fn get_payment_link_in_tenant(
        &self,
        app_id: &str,
        org_id: &str,
        id: &str,
    ) -> Result<Option<PaymentLink>> {
        let filter = doc! {
            "_id": id,
            "app_id": app_id,
            "org_id": org_id
        };
        Ok(self.payment_link_collection.find_one(filter, None).await?)
    }

    pub async fn get_payment_link_by_razorpay_id(
        &self,
        razorpay_payment_link_id: &str,
    ) -> Result<Option<PaymentLink>> {
        let filter = doc! { "razorpay_payment_link_id": razorpay_payment_link_id };
        Ok(self.payment_link_collection.find_one(filter, None).await?)
    }

    pub async fn update_payment_link_status_by_razorpay_id(
        &self,
        razorpay_payment_link_id: &str,
        status: PaymentLinkStatus,
    ) -> Result<()> {
        let filter = doc! { "razorpay_payment_link_id": razorpay_payment_link_id };
        let update = doc! {
            "$set": {
                "status": mongodb::bson::to_bson(&status)?,
                "updated_at": mongodb::bson::DateTime::now()
            }
        };
        self.payment_link_collection
            .update_one(filter, update, None)
            .await?;
        Ok(())
    }

    pub async fn update_payment_link_in_tenant(
        &self,
        app_id: &str,
        org_id: &str,
        id: &str,
        update: mongodb::bson::Document,
    ) -> Result<()> {
        let filter = doc! {
            "_id": id,
            "app_id": app_id,
            "org_id": org_id
        };
        self.payment_link_collection
            .update_one(filter, doc! { "$set": update }, None)
            .await?;
        Ok(())
    }

    pub async fn list_payment_links_in_tenant(
        &self,
        app_id: &str,
        org_id: &str,
        status_filter: Option<PaymentLinkStatus>,
        limit: i64,
        offset: u64,
    ) -> Result<(Vec<PaymentLink>, i64)> {
        use futures::TryStreamExt;
        use mongodb::options::FindOptions;

        let mut filter = doc! {
            "app_id": app_id,
            "org_id": org_id
        };

        if let Some(status) = status_filter {
            filter.insert("status", mongodb::bson::to_bson(&status)?);
        }

        let total_count = self
            .payment_link_collection
            .count_documents(filter.clone(), None)
            .await? as i64;

        let options = FindOptions::builder()
            .sort(doc! { "created_at": -1 })
            .skip(offset)
            .limit(limit)
            .build();

        let cursor = self
            .payment_link_collection
            .find(filter, Some(options))
            .await?;

        let links: Vec<PaymentLink> = cursor.try_collect().await?;
        Ok((links, total_count))
    }
}
