//! Migration to add multi-tenancy support (app_id, org_id) to existing data.
//!
//! This migration:
//! 1. Creates a "Default" organization for each registered client (app)
//! 2. Updates all existing users to belong to the default app and org
//!
//! Run this migration once after deploying the multi-tenant schema changes.

use chrono::Utc;
use futures::TryStreamExt;
use mongodb::bson::doc;
use service_core::error::AppError;
use uuid::Uuid;

use crate::{models::Organization, services::MongoDb};

/// Default placeholder app_id for legacy users (before multi-tenancy)
pub const LEGACY_APP_ID: &str = "00000000-0000-0000-0000-000000000000";

/// Default placeholder org_id for legacy users (before multi-tenancy)
pub const LEGACY_ORG_ID: &str = "00000000-0000-0000-0000-000000000000";

/// Result of the migration
#[derive(Debug)]
pub struct MigrationResult {
    pub organizations_created: u64,
    pub users_updated: u64,
    pub clients_processed: u64,
}

/// Migrate existing data to multi-tenant schema.
///
/// This function is idempotent - running it multiple times is safe.
///
/// # Steps
/// 1. Create a legacy/default organization for users without a specific app
/// 2. For each registered client, create a "Default" organization
/// 3. Update all users without app_id/org_id to use the legacy values
///
/// # Usage
/// ```ignore
/// let db = MongoDb::connect(&uri, &database).await?;
/// let result = migrate_to_multi_tenant(&db).await?;
/// println!("Migration complete: {:?}", result);
/// ```
pub async fn migrate_to_multi_tenant(db: &MongoDb) -> Result<MigrationResult, AppError> {
    tracing::info!("Starting multi-tenant migration");

    let mut result = MigrationResult {
        organizations_created: 0,
        users_updated: 0,
        clients_processed: 0,
    };

    // Step 1: Create legacy organization for users without a specific app
    let legacy_org_exists = db
        .find_organization_by_id(LEGACY_ORG_ID)
        .await
        .map_err(AppError::from)?
        .is_some();

    if !legacy_org_exists {
        let legacy_org = Organization {
            id: LEGACY_ORG_ID.to_string(),
            org_id: LEGACY_ORG_ID.to_string(),
            app_id: LEGACY_APP_ID.to_string(),
            name: "Legacy Default".to_string(),
            settings: crate::models::OrgSettings::default(),
            auth_policy: crate::models::AuthPolicy::default(),
            enabled: true,
            created_at: Utc::now(),
            updated_at: Utc::now(),
        };

        db.organizations()
            .insert_one(&legacy_org, None)
            .await
            .map_err(AppError::from)?;

        result.organizations_created += 1;
        tracing::info!("Created legacy default organization");
    }

    // Step 2: Create default organizations for each registered client
    let mut cursor = db
        .clients()
        .find(doc! {}, None)
        .await
        .map_err(AppError::from)?;

    while let Some(client) = cursor.try_next().await.map_err(AppError::from)? {
        result.clients_processed += 1;

        let client_id = client.client_id.clone();
        let app_name = client.app_name.clone();

        // Check if default org already exists for this client
        let existing_org = db
            .organizations()
            .find_one(
                doc! {
                    "app_id": &client_id,
                    "name": "Default"
                },
                None,
            )
            .await
            .map_err(AppError::from)?;

        if existing_org.is_none() {
            let org_id = Uuid::new_v4().to_string();
            let org = Organization {
                id: org_id.clone(),
                org_id,
                app_id: client_id.clone(),
                name: "Default".to_string(),
                settings: crate::models::OrgSettings::default(),
                auth_policy: crate::models::AuthPolicy::default(),
                enabled: true,
                created_at: Utc::now(),
                updated_at: Utc::now(),
            };

            db.organizations()
                .insert_one(&org, None)
                .await
                .map_err(AppError::from)?;

            result.organizations_created += 1;
            tracing::info!(
                client_id = %client_id,
                app_name = %app_name,
                org_id = %org.org_id,
                "Created default organization for client"
            );
        }
    }

    // Step 3: Update users without app_id/org_id to use legacy values
    let update_result = db
        .users()
        .update_many(
            doc! {
                "$or": [
                    { "app_id": { "$exists": false } },
                    { "org_id": { "$exists": false } }
                ]
            },
            doc! {
                "$set": {
                    "app_id": LEGACY_APP_ID,
                    "org_id": LEGACY_ORG_ID,
                    "updated_at": mongodb::bson::DateTime::from_chrono(Utc::now())
                }
            },
            None,
        )
        .await
        .map_err(AppError::from)?;

    result.users_updated = update_result.modified_count;

    tracing::info!(
        organizations_created = result.organizations_created,
        users_updated = result.users_updated,
        clients_processed = result.clients_processed,
        "Multi-tenant migration complete"
    );

    Ok(result)
}

/// Check if migration is needed by looking for users without tenant context.
pub async fn is_migration_needed(db: &MongoDb) -> Result<bool, AppError> {
    let count = db
        .users()
        .count_documents(
            doc! {
                "$or": [
                    { "app_id": { "$exists": false } },
                    { "org_id": { "$exists": false } }
                ]
            },
            None,
        )
        .await
        .map_err(AppError::from)?;

    Ok(count > 0)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_legacy_ids_are_valid_uuids() {
        // Ensure legacy IDs are valid UUIDs (all zeros)
        assert!(Uuid::parse_str(LEGACY_APP_ID).is_ok());
        assert!(Uuid::parse_str(LEGACY_ORG_ID).is_ok());
    }
}
