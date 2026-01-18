//! Organization administration handlers.
//!
//! These endpoints allow app clients to manage organizations within their app.
//! All endpoints require app authentication (app token with valid client_id).

use chrono::Utc;
use mongodb::bson::doc;
use service_core::{
    axum::{extract::State, http::StatusCode, response::IntoResponse, Json},
    error::AppError,
};

use crate::{
    dtos::admin::{
        CreateOrganizationRequest, CreateOrganizationResponse, ListOrganizationsResponse,
        UpdateAuthPolicyRequest, UpdateOrganizationRequest,
    },
    middleware::app_auth::CurrentApp,
    models::{Organization, SanitizedOrganization},
    utils::ValidatedJson,
    AppState,
};

/// Create a new organization under the calling app.
///
/// The app_id is derived from the authenticated app token.
#[utoipa::path(
    post,
    path = "/admin/orgs",
    request_body = CreateOrganizationRequest,
    responses(
        (status = 201, description = "Organization created successfully", body = CreateOrganizationResponse),
        (status = 401, description = "Unauthorized - invalid app token"),
        (status = 409, description = "Organization name already exists in this app"),
        (status = 422, description = "Validation error")
    ),
    security(("app_token" = [])),
    tag = "Organization Admin"
)]
pub async fn create_organization(
    State(state): State<AppState>,
    app: CurrentApp,
    ValidatedJson(req): ValidatedJson<CreateOrganizationRequest>,
) -> Result<impl IntoResponse, AppError> {
    let app_id = app.0.client_id.clone();

    // Check if org name already exists in this app
    let existing = state
        .db
        .organizations()
        .find_one(
            doc! {
                "app_id": &app_id,
                "name": &req.name
            },
            None,
        )
        .await
        .map_err(AppError::from)?;

    if existing.is_some() {
        return Err(AppError::Conflict(anyhow::anyhow!(
            "Organization '{}' already exists in this app",
            req.name
        )));
    }

    // Create organization with optional custom settings/policies
    let org = if let (Some(settings), Some(auth_policy)) = (req.settings, req.auth_policy) {
        Organization::with_config(app_id.clone(), req.name.clone(), settings, auth_policy)
    } else {
        Organization::new(app_id.clone(), req.name.clone())
    };

    state
        .db
        .organizations()
        .insert_one(&org, None)
        .await
        .map_err(AppError::from)?;

    tracing::info!(
        org_id = %org.org_id,
        app_id = %app_id,
        name = %org.name,
        "Organization created"
    );

    Ok((
        StatusCode::CREATED,
        Json(CreateOrganizationResponse {
            org_id: org.org_id,
            app_id,
            name: org.name,
            created_at: org.created_at,
        }),
    ))
}

/// List all organizations for the calling app.
#[utoipa::path(
    get,
    path = "/admin/orgs",
    responses(
        (status = 200, description = "List of organizations", body = ListOrganizationsResponse),
        (status = 401, description = "Unauthorized - invalid app token")
    ),
    security(("app_token" = [])),
    tag = "Organization Admin"
)]
pub async fn list_organizations(
    State(state): State<AppState>,
    app: CurrentApp,
) -> Result<impl IntoResponse, AppError> {
    let app_id = app.0.client_id;

    let orgs = state
        .db
        .find_organizations_by_app(&app_id)
        .await
        .map_err(AppError::from)?;

    let sanitized: Vec<SanitizedOrganization> = orgs.into_iter().map(|o| o.into()).collect();
    let total = sanitized.len();

    Ok(Json(ListOrganizationsResponse {
        organizations: sanitized,
        total,
    }))
}

/// Get a specific organization by ID.
#[utoipa::path(
    get,
    path = "/admin/orgs/{org_id}",
    params(
        ("org_id" = String, Path, description = "Organization ID")
    ),
    responses(
        (status = 200, description = "Organization details", body = SanitizedOrganization),
        (status = 401, description = "Unauthorized - invalid app token"),
        (status = 404, description = "Organization not found")
    ),
    security(("app_token" = [])),
    tag = "Organization Admin"
)]
pub async fn get_organization(
    State(state): State<AppState>,
    app: CurrentApp,
    service_core::axum::extract::Path(org_id): service_core::axum::extract::Path<String>,
) -> Result<impl IntoResponse, AppError> {
    let app_id = app.0.client_id;

    let org = state
        .db
        .find_organization_in_app(&app_id, &org_id)
        .await
        .map_err(AppError::from)?
        .ok_or_else(|| AppError::NotFound(anyhow::anyhow!("Organization not found")))?;

    Ok(Json(SanitizedOrganization::from(org)))
}

/// Update an organization.
#[utoipa::path(
    put,
    path = "/admin/orgs/{org_id}",
    params(
        ("org_id" = String, Path, description = "Organization ID")
    ),
    request_body = UpdateOrganizationRequest,
    responses(
        (status = 200, description = "Organization updated", body = SanitizedOrganization),
        (status = 401, description = "Unauthorized - invalid app token"),
        (status = 404, description = "Organization not found"),
        (status = 409, description = "Organization name already exists")
    ),
    security(("app_token" = [])),
    tag = "Organization Admin"
)]
pub async fn update_organization(
    State(state): State<AppState>,
    app: CurrentApp,
    service_core::axum::extract::Path(org_id): service_core::axum::extract::Path<String>,
    ValidatedJson(req): ValidatedJson<UpdateOrganizationRequest>,
) -> Result<impl IntoResponse, AppError> {
    let app_id = app.0.client_id;

    // Verify org exists and belongs to this app
    let org = state
        .db
        .find_organization_in_app(&app_id, &org_id)
        .await
        .map_err(AppError::from)?
        .ok_or_else(|| AppError::NotFound(anyhow::anyhow!("Organization not found")))?;

    // If renaming, check for conflicts
    if let Some(ref new_name) = req.name {
        if new_name != &org.name {
            let existing = state
                .db
                .organizations()
                .find_one(
                    doc! {
                        "app_id": &app_id,
                        "name": new_name,
                        "org_id": { "$ne": &org_id }
                    },
                    None,
                )
                .await
                .map_err(AppError::from)?;

            if existing.is_some() {
                return Err(AppError::Conflict(anyhow::anyhow!(
                    "Organization '{}' already exists in this app",
                    new_name
                )));
            }
        }
    }

    // Build update document
    let mut update_doc = doc! { "updated_at": mongodb::bson::DateTime::from_chrono(Utc::now()) };

    if let Some(name) = req.name {
        update_doc.insert("name", name);
    }
    if let Some(enabled) = req.enabled {
        update_doc.insert("enabled", enabled);
    }
    if let Some(settings) = req.settings {
        update_doc.insert(
            "settings",
            mongodb::bson::to_bson(&settings).map_err(|e| {
                AppError::InternalError(anyhow::anyhow!("Failed to serialize settings: {}", e))
            })?,
        );
    }

    state
        .db
        .organizations()
        .update_one(
            doc! { "app_id": &app_id, "org_id": &org_id },
            doc! { "$set": update_doc },
            None,
        )
        .await
        .map_err(AppError::from)?;

    // Fetch updated org
    let updated_org = state
        .db
        .find_organization_in_app(&app_id, &org_id)
        .await
        .map_err(AppError::from)?
        .ok_or_else(|| AppError::NotFound(anyhow::anyhow!("Organization not found")))?;

    tracing::info!(org_id = %org_id, app_id = %app_id, "Organization updated");

    Ok(Json(SanitizedOrganization::from(updated_org)))
}

/// Soft delete an organization (disable it).
#[utoipa::path(
    delete,
    path = "/admin/orgs/{org_id}",
    params(
        ("org_id" = String, Path, description = "Organization ID")
    ),
    responses(
        (status = 204, description = "Organization deleted"),
        (status = 401, description = "Unauthorized - invalid app token"),
        (status = 404, description = "Organization not found")
    ),
    security(("app_token" = [])),
    tag = "Organization Admin"
)]
pub async fn delete_organization(
    State(state): State<AppState>,
    app: CurrentApp,
    service_core::axum::extract::Path(org_id): service_core::axum::extract::Path<String>,
) -> Result<impl IntoResponse, AppError> {
    let app_id = app.0.client_id;

    // Verify org exists and belongs to this app
    let result = state
        .db
        .organizations()
        .update_one(
            doc! { "app_id": &app_id, "org_id": &org_id },
            doc! { "$set": {
                "enabled": false,
                "updated_at": mongodb::bson::DateTime::from_chrono(Utc::now())
            }},
            None,
        )
        .await
        .map_err(AppError::from)?;

    if result.matched_count == 0 {
        return Err(AppError::NotFound(anyhow::anyhow!(
            "Organization not found"
        )));
    }

    tracing::info!(org_id = %org_id, app_id = %app_id, "Organization soft-deleted");

    Ok(StatusCode::NO_CONTENT)
}

/// Update auth policies for an organization.
#[utoipa::path(
    put,
    path = "/admin/orgs/{org_id}/auth-policy",
    params(
        ("org_id" = String, Path, description = "Organization ID")
    ),
    request_body = UpdateAuthPolicyRequest,
    responses(
        (status = 200, description = "Auth policy updated", body = SanitizedOrganization),
        (status = 401, description = "Unauthorized - invalid app token"),
        (status = 404, description = "Organization not found")
    ),
    security(("app_token" = [])),
    tag = "Organization Admin"
)]
pub async fn update_auth_policy(
    State(state): State<AppState>,
    app: CurrentApp,
    service_core::axum::extract::Path(org_id): service_core::axum::extract::Path<String>,
    ValidatedJson(req): ValidatedJson<UpdateAuthPolicyRequest>,
) -> Result<impl IntoResponse, AppError> {
    let app_id = app.0.client_id;

    let policy_bson = mongodb::bson::to_bson(&req.auth_policy).map_err(|e| {
        AppError::InternalError(anyhow::anyhow!("Failed to serialize auth policy: {}", e))
    })?;

    let result = state
        .db
        .organizations()
        .update_one(
            doc! { "app_id": &app_id, "org_id": &org_id },
            doc! { "$set": {
                "auth_policy": policy_bson,
                "updated_at": mongodb::bson::DateTime::from_chrono(Utc::now())
            }},
            None,
        )
        .await
        .map_err(AppError::from)?;

    if result.matched_count == 0 {
        return Err(AppError::NotFound(anyhow::anyhow!(
            "Organization not found"
        )));
    }

    // Fetch updated org
    let updated_org = state
        .db
        .find_organization_in_app(&app_id, &org_id)
        .await
        .map_err(AppError::from)?
        .ok_or_else(|| AppError::NotFound(anyhow::anyhow!("Organization not found")))?;

    tracing::info!(org_id = %org_id, app_id = %app_id, "Auth policy updated");

    Ok(Json(SanitizedOrganization::from(updated_org)))
}
