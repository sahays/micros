use crate::models::{RazorpayPlan, RazorpaySubscription, SubscriptionStatus};
use anyhow::Result;
use mongodb::bson::doc;

use super::repository::PaymentRepository;

// Plan operations
impl PaymentRepository {
    pub async fn create_plan(&self, plan: RazorpayPlan) -> Result<()> {
        self.plan_collection.insert_one(plan, None).await?;
        Ok(())
    }

    pub async fn get_plan_in_tenant(
        &self,
        app_id: &str,
        tenant_id: &str,
        id: &str,
    ) -> Result<Option<RazorpayPlan>> {
        let filter = doc! {
            "_id": id,
            "app_id": app_id,
            "tenant_id": tenant_id
        };
        Ok(self.plan_collection.find_one(filter, None).await?)
    }

    pub async fn list_plans_in_tenant(
        &self,
        app_id: &str,
        tenant_id: &str,
        limit: i64,
        offset: u64,
    ) -> Result<(Vec<RazorpayPlan>, i64)> {
        use futures::TryStreamExt;
        use mongodb::options::FindOptions;

        let filter = doc! {
            "app_id": app_id,
            "tenant_id": tenant_id
        };

        let total_count = self
            .plan_collection
            .count_documents(filter.clone(), None)
            .await? as i64;

        let options = FindOptions::builder()
            .sort(doc! { "created_at": -1 })
            .skip(offset)
            .limit(limit)
            .build();

        let cursor = self.plan_collection.find(filter, Some(options)).await?;

        let plans: Vec<RazorpayPlan> = cursor.try_collect().await?;
        Ok((plans, total_count))
    }
}

// Subscription operations
impl PaymentRepository {
    pub async fn create_subscription(&self, subscription: RazorpaySubscription) -> Result<()> {
        self.subscription_collection
            .insert_one(subscription, None)
            .await?;
        Ok(())
    }

    pub async fn get_subscription_in_tenant(
        &self,
        app_id: &str,
        tenant_id: &str,
        id: &str,
    ) -> Result<Option<RazorpaySubscription>> {
        let filter = doc! {
            "_id": id,
            "app_id": app_id,
            "tenant_id": tenant_id
        };
        Ok(self.subscription_collection.find_one(filter, None).await?)
    }

    pub async fn get_subscription_by_razorpay_id(
        &self,
        razorpay_subscription_id: &str,
    ) -> Result<Option<RazorpaySubscription>> {
        let filter = doc! { "razorpay_subscription_id": razorpay_subscription_id };
        Ok(self.subscription_collection.find_one(filter, None).await?)
    }

    pub async fn update_subscription_status_by_razorpay_id(
        &self,
        razorpay_subscription_id: &str,
        status: SubscriptionStatus,
    ) -> Result<()> {
        let filter = doc! { "razorpay_subscription_id": razorpay_subscription_id };
        let update = doc! {
            "$set": {
                "status": mongodb::bson::to_bson(&status)?,
                "updated_at": mongodb::bson::DateTime::now()
            }
        };
        self.subscription_collection
            .update_one(filter, update, None)
            .await?;
        Ok(())
    }

    pub async fn update_subscription_in_tenant(
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
        self.subscription_collection
            .update_one(filter, doc! { "$set": update }, None)
            .await?;
        Ok(())
    }

    pub async fn increment_subscription_charge_count(
        &self,
        razorpay_subscription_id: &str,
    ) -> Result<()> {
        let filter = doc! { "razorpay_subscription_id": razorpay_subscription_id };
        let update = doc! {
            "$inc": { "charge_count": 1, "paid_count": 1 },
            "$set": { "updated_at": mongodb::bson::DateTime::now() }
        };
        self.subscription_collection
            .update_one(filter, update, None)
            .await?;
        Ok(())
    }

    #[allow(clippy::too_many_arguments)]
    pub async fn list_subscriptions_in_tenant(
        &self,
        app_id: &str,
        tenant_id: &str,
        customer_id: Option<&str>,
        plan_id: Option<&str>,
        status_filter: Option<SubscriptionStatus>,
        limit: i64,
        offset: u64,
    ) -> Result<(Vec<RazorpaySubscription>, i64)> {
        use futures::TryStreamExt;
        use mongodb::options::FindOptions;

        let mut filter = doc! {
            "app_id": app_id,
            "tenant_id": tenant_id
        };

        if let Some(cid) = customer_id {
            filter.insert("customer_id", cid);
        }
        if let Some(pid) = plan_id {
            filter.insert("plan_id", pid);
        }
        if let Some(status) = status_filter {
            filter.insert("status", mongodb::bson::to_bson(&status)?);
        }

        let total_count = self
            .subscription_collection
            .count_documents(filter.clone(), None)
            .await? as i64;

        let options = FindOptions::builder()
            .sort(doc! { "created_at": -1 })
            .skip(offset)
            .limit(limit)
            .build();

        let cursor = self
            .subscription_collection
            .find(filter, Some(options))
            .await?;

        let subscriptions: Vec<RazorpaySubscription> = cursor.try_collect().await?;
        Ok((subscriptions, total_count))
    }
}
