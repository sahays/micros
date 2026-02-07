use crate::models::{LinkedAccount, LinkedAccountStatus};
use anyhow::Result;
use mongodb::bson::doc;

use super::repository::PaymentRepository;

impl PaymentRepository {
    pub async fn create_linked_account(&self, account: LinkedAccount) -> Result<()> {
        self.linked_account_collection
            .insert_one(account, None)
            .await?;
        Ok(())
    }

    pub async fn get_linked_account_in_tenant(
        &self,
        app_id: &str,
        org_id: &str,
        id: &str,
    ) -> Result<Option<LinkedAccount>> {
        let filter = doc! {
            "_id": id,
            "app_id": app_id,
            "org_id": org_id
        };
        Ok(self
            .linked_account_collection
            .find_one(filter, None)
            .await?)
    }

    pub async fn get_linked_account_by_org_in_tenant(
        &self,
        app_id: &str,
        org_id: &str,
    ) -> Result<Option<LinkedAccount>> {
        let filter = doc! {
            "app_id": app_id,
            "org_id": org_id
        };
        Ok(self
            .linked_account_collection
            .find_one(filter, None)
            .await?)
    }

    pub async fn update_linked_account_in_tenant(
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
        self.linked_account_collection
            .update_one(filter, doc! { "$set": update }, None)
            .await?;
        Ok(())
    }

    pub async fn update_linked_account_status_by_razorpay_id(
        &self,
        razorpay_account_id: &str,
        status: LinkedAccountStatus,
    ) -> Result<()> {
        let filter = doc! { "razorpay_account_id": razorpay_account_id };
        let update = doc! {
            "$set": {
                "status": mongodb::bson::to_bson(&status)?,
                "updated_at": mongodb::bson::DateTime::now()
            }
        };
        self.linked_account_collection
            .update_one(filter, update, None)
            .await?;
        Ok(())
    }

    pub async fn list_linked_accounts_in_tenant(
        &self,
        app_id: &str,
        org_id: &str,
        status_filter: Option<LinkedAccountStatus>,
        limit: i64,
        offset: u64,
    ) -> Result<(Vec<LinkedAccount>, i64)> {
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
            .linked_account_collection
            .count_documents(filter.clone(), None)
            .await? as i64;

        let options = FindOptions::builder()
            .sort(doc! { "created_at": -1 })
            .skip(offset)
            .limit(limit)
            .build();

        let cursor = self
            .linked_account_collection
            .find(filter, Some(options))
            .await?;

        let accounts: Vec<LinkedAccount> = cursor.try_collect().await?;
        Ok((accounts, total_count))
    }
}
