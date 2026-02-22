use crate::models::RazorpayCustomer;
use anyhow::Result;
use mongodb::bson::doc;

use super::repository::PaymentRepository;

impl PaymentRepository {
    pub async fn create_customer(&self, customer: RazorpayCustomer) -> Result<()> {
        self.customer_collection.insert_one(customer, None).await?;
        Ok(())
    }

    pub async fn get_customer_in_tenant(
        &self,
        app_id: &str,
        tenant_id: &str,
        id: &str,
    ) -> Result<Option<RazorpayCustomer>> {
        let filter = doc! {
            "_id": id,
            "app_id": app_id,
            "tenant_id": tenant_id
        };
        Ok(self.customer_collection.find_one(filter, None).await?)
    }

    pub async fn get_customer_by_user_in_tenant(
        &self,
        app_id: &str,
        tenant_id: &str,
        user_id: &str,
    ) -> Result<Option<RazorpayCustomer>> {
        let filter = doc! {
            "app_id": app_id,
            "tenant_id": tenant_id,
            "user_id": user_id
        };
        Ok(self.customer_collection.find_one(filter, None).await?)
    }

    pub async fn update_customer_in_tenant(
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
        self.customer_collection
            .update_one(filter, doc! { "$set": update }, None)
            .await?;
        Ok(())
    }

    pub async fn list_customers_in_tenant(
        &self,
        app_id: &str,
        tenant_id: &str,
        limit: i64,
        offset: u64,
    ) -> Result<(Vec<RazorpayCustomer>, i64)> {
        use futures::TryStreamExt;
        use mongodb::options::FindOptions;

        let filter = doc! {
            "app_id": app_id,
            "tenant_id": tenant_id
        };

        let total_count = self
            .customer_collection
            .count_documents(filter.clone(), None)
            .await? as i64;

        let options = FindOptions::builder()
            .sort(doc! { "created_at": -1 })
            .skip(offset)
            .limit(limit)
            .build();

        let cursor = self.customer_collection.find(filter, Some(options)).await?;

        let customers: Vec<RazorpayCustomer> = cursor.try_collect().await?;
        Ok((customers, total_count))
    }
}
