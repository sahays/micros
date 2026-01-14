use crate::models::{Transaction, TransactionStatus, PaymentMethod};
use mongodb::{Database, Collection, bson::{doc, Uuid as BsonUuid}};
use uuid::Uuid;
use anyhow::Result;

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

    pub async fn create_transaction(&self, transaction: Transaction) -> Result<()> {
        self.transaction_collection.insert_one(transaction, None).await?;
        Ok(())
    }

    pub async fn get_transaction(&self, id: Uuid) -> Result<Option<Transaction>> {
        let filter = doc! { "_id": BsonUuid::from_bytes(id.into_bytes()) };
        let transaction = self.transaction_collection.find_one(filter, None).await?;
        Ok(transaction)
    }

    pub async fn update_transaction_status(&self, id: Uuid, status: TransactionStatus) -> Result<()> {
        let filter = doc! { "_id": BsonUuid::from_bytes(id.into_bytes()) };
        let update = doc! { 
            "$set": { 
                "status": mongodb::bson::to_bson(&status)?,
                "updated_at": mongodb::bson::DateTime::now()
            } 
        };
        self.transaction_collection.update_one(filter, update, None).await?;
        Ok(())
    }

    pub async fn save_payment_method(&self, method: PaymentMethod) -> Result<()> {
        self.payment_method_collection.insert_one(method, None).await?;
        Ok(())
    }
}
