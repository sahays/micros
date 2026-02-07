use crate::models::{Settlement, SettlementStatus, SettlementType};
use anyhow::Result;
use mongodb::bson::doc;

use super::repository::PaymentRepository;

impl PaymentRepository {
    pub async fn create_settlement(&self, settlement: Settlement) -> Result<()> {
        self.settlement_collection
            .insert_one(settlement, None)
            .await?;
        Ok(())
    }

    pub async fn get_settlement_in_tenant(
        &self,
        app_id: &str,
        org_id: &str,
        id: &str,
    ) -> Result<Option<Settlement>> {
        let filter = doc! {
            "_id": id,
            "app_id": app_id,
            "org_id": org_id
        };
        Ok(self.settlement_collection.find_one(filter, None).await?)
    }

    pub async fn get_settlement_by_razorpay_id(
        &self,
        razorpay_settlement_id: &str,
    ) -> Result<Option<Settlement>> {
        let filter = doc! { "razorpay_settlement_id": razorpay_settlement_id };
        Ok(self.settlement_collection.find_one(filter, None).await?)
    }

    pub async fn update_settlement_by_razorpay_id(
        &self,
        razorpay_settlement_id: &str,
        update: mongodb::bson::Document,
    ) -> Result<()> {
        let filter = doc! { "razorpay_settlement_id": razorpay_settlement_id };
        self.settlement_collection
            .update_one(filter, doc! { "$set": update }, None)
            .await?;
        Ok(())
    }

    #[allow(clippy::too_many_arguments)]
    pub async fn list_settlements_in_tenant(
        &self,
        app_id: &str,
        org_id: &str,
        linked_account_id: Option<&str>,
        status_filter: Option<SettlementStatus>,
        type_filter: Option<SettlementType>,
        limit: i64,
        offset: u64,
    ) -> Result<(Vec<Settlement>, i64)> {
        use futures::TryStreamExt;
        use mongodb::options::FindOptions;

        let mut filter = doc! {
            "app_id": app_id,
            "org_id": org_id
        };

        if let Some(la_id) = linked_account_id {
            filter.insert("linked_account_id", la_id);
        }
        if let Some(status) = status_filter {
            filter.insert("status", mongodb::bson::to_bson(&status)?);
        }
        if let Some(stype) = type_filter {
            filter.insert("settlement_type", mongodb::bson::to_bson(&stype)?);
        }

        let total_count = self
            .settlement_collection
            .count_documents(filter.clone(), None)
            .await? as i64;

        let options = FindOptions::builder()
            .sort(doc! { "created_at": -1 })
            .skip(offset)
            .limit(limit)
            .build();

        let cursor = self
            .settlement_collection
            .find(filter, Some(options))
            .await?;

        let settlements: Vec<Settlement> = cursor.try_collect().await?;
        Ok((settlements, total_count))
    }
}
