//! gRPC handlers for linked account operations.

use crate::grpc::helpers::{
    check_feature_flag, check_razorpay_configured, datetime_to_timestamp, extract_tenant_context,
};
use crate::grpc::proto::*;
use crate::models;
use crate::services::razorpay_accounts;
use crate::startup::AppState;
use mongodb::bson::DateTime;
use tonic::{Request, Response, Status};
use uuid::Uuid;

pub async fn create_linked_account(
    state: &AppState,
    request: Request<CreateLinkedAccountRequest>,
) -> Result<Response<CreateLinkedAccountResponse>, Status> {
    check_feature_flag(
        state.config.feature_flags.razorpay_route_enabled,
        "Razorpay Route",
    )?;
    check_razorpay_configured(&state.razorpay)?;

    let tenant = extract_tenant_context(&request)?;
    let req = request.into_inner();

    // Check if org already has a linked account
    if let Some(_existing) = state
        .repository
        .get_linked_account_by_org_in_tenant(&tenant.app_id, &tenant.tenant_id)
        .await
        .map_err(|e| {
            tracing::error!(error = %e, "Failed to check existing linked account");
            Status::internal("Failed to check existing linked account")
        })?
    {
        return Err(Status::already_exists(
            "Organization already has a linked account",
        ));
    }

    let bank = req
        .bank_account
        .ok_or_else(|| Status::invalid_argument("Bank account is required"))?;
    let legal = req
        .legal_info
        .ok_or_else(|| Status::invalid_argument("Legal info is required"))?;

    // Create account in Razorpay
    let rz_request = razorpay_accounts::CreateAccountRequest {
        email: req.email.clone(),
        phone: None,
        account_type: "route".to_string(),
        legal_business_name: legal.legal_business_name.clone(),
        business_type: legal.business_type.clone(),
        legal_info: Some(razorpay_accounts::CreateAccountLegalInfo {
            pan: legal.pan.clone(),
            gst: legal.gst.clone(),
        }),
    };

    let rz_response = state
        .razorpay
        .create_account(rz_request)
        .await
        .map_err(|e| {
            tracing::error!(error = %e, "Failed to create Razorpay account");
            Status::internal(format!("Failed to create linked account: {}", e))
        })?;

    let now = DateTime::now();
    let account = models::LinkedAccount {
        id: Uuid::new_v4().to_string(),
        app_id: tenant.app_id.clone(),
        tenant_id: tenant.tenant_id.clone(),
        razorpay_account_id: rz_response.id,
        name: req.name,
        email: req.email,
        status: models::LinkedAccountStatus::Created,
        commission: req.commission.map(|c| models::CommissionConfig {
            commission_type: match c.commission_type {
                x if x == CommissionType::Flat as i32 => models::CommissionType::Flat,
                _ => models::CommissionType::Percentage,
            },
            value: c.value,
        }),
        bank_account: Some(models::BankAccount {
            account_holder_name: bank.account_holder_name,
            account_number: bank.account_number,
            ifsc_code: bank.ifsc_code,
            account_type: bank.account_type,
        }),
        legal_info: Some(models::LegalInfo {
            legal_business_name: legal.legal_business_name,
            business_type: legal.business_type,
            pan: legal.pan,
            gst: legal.gst,
        }),
        created_at: now,
        updated_at: now,
    };

    state
        .repository
        .create_linked_account(account.clone())
        .await
        .map_err(|e| {
            tracing::error!(error = %e, "Failed to save linked account");
            Status::internal("Failed to save linked account")
        })?;

    Ok(Response::new(CreateLinkedAccountResponse {
        linked_account: Some(linked_account_to_proto(account)),
    }))
}

pub async fn get_linked_account(
    state: &AppState,
    request: Request<GetLinkedAccountRequest>,
) -> Result<Response<GetLinkedAccountResponse>, Status> {
    let tenant = extract_tenant_context(&request)?;
    let req = request.into_inner();

    let account = state
        .repository
        .get_linked_account_in_tenant(&tenant.app_id, &tenant.tenant_id, &req.linked_account_id)
        .await
        .map_err(|e| {
            tracing::error!(error = %e, "Failed to fetch linked account");
            Status::internal("Failed to fetch linked account")
        })?
        .ok_or_else(|| Status::not_found("Linked account not found"))?;

    Ok(Response::new(GetLinkedAccountResponse {
        linked_account: Some(linked_account_to_proto(account)),
    }))
}

pub async fn update_linked_account(
    state: &AppState,
    request: Request<UpdateLinkedAccountRequest>,
) -> Result<Response<UpdateLinkedAccountResponse>, Status> {
    check_feature_flag(
        state.config.feature_flags.razorpay_route_enabled,
        "Razorpay Route",
    )?;

    let tenant = extract_tenant_context(&request)?;
    let req = request.into_inner();

    let mut update = mongodb::bson::doc! { "updated_at": DateTime::now() };
    if let Some(name) = &req.name {
        update.insert("name", name);
    }
    if let Some(email) = &req.email {
        update.insert("email", email);
    }

    state
        .repository
        .update_linked_account_in_tenant(
            &tenant.app_id,
            &tenant.tenant_id,
            &req.linked_account_id,
            update,
        )
        .await
        .map_err(|e| {
            tracing::error!(error = %e, "Failed to update linked account");
            Status::internal("Failed to update linked account")
        })?;

    let account = state
        .repository
        .get_linked_account_in_tenant(&tenant.app_id, &tenant.tenant_id, &req.linked_account_id)
        .await
        .map_err(|e| {
            tracing::error!(error = %e, "Failed to fetch updated linked account");
            Status::internal("Failed to fetch linked account")
        })?
        .ok_or_else(|| Status::not_found("Linked account not found"))?;

    Ok(Response::new(UpdateLinkedAccountResponse {
        linked_account: Some(linked_account_to_proto(account)),
    }))
}

pub async fn list_linked_accounts(
    state: &AppState,
    request: Request<ListLinkedAccountsRequest>,
) -> Result<Response<ListLinkedAccountsResponse>, Status> {
    let tenant = extract_tenant_context(&request)?;
    let req = request.into_inner();

    let status_filter = req.status.and_then(proto_to_linked_account_status);
    let limit = req.limit.clamp(1, 100) as i64;
    let offset = req.offset.max(0) as u64;

    let (accounts, total_count) = state
        .repository
        .list_linked_accounts_in_tenant(
            &tenant.app_id,
            &tenant.tenant_id,
            status_filter,
            limit,
            offset,
        )
        .await
        .map_err(|e| {
            tracing::error!(error = %e, "Failed to list linked accounts");
            Status::internal("Failed to list linked accounts")
        })?;

    Ok(Response::new(ListLinkedAccountsResponse {
        linked_accounts: accounts.into_iter().map(linked_account_to_proto).collect(),
        total_count,
    }))
}

pub async fn update_commission_config(
    state: &AppState,
    request: Request<UpdateCommissionConfigRequest>,
) -> Result<Response<UpdateCommissionConfigResponse>, Status> {
    check_feature_flag(
        state.config.feature_flags.razorpay_route_enabled,
        "Razorpay Route",
    )?;

    let tenant = extract_tenant_context(&request)?;
    let req = request.into_inner();

    let commission = req
        .commission
        .ok_or_else(|| Status::invalid_argument("Commission configuration is required"))?;

    let commission_model = models::CommissionConfig {
        commission_type: match commission.commission_type {
            x if x == CommissionType::Flat as i32 => models::CommissionType::Flat,
            _ => models::CommissionType::Percentage,
        },
        value: commission.value,
    };

    let update = mongodb::bson::doc! {
        "commission": mongodb::bson::to_bson(&commission_model).map_err(|e| {
            tracing::error!(error = %e, "Failed to serialize commission");
            Status::internal("Failed to serialize commission")
        })?,
        "updated_at": DateTime::now()
    };

    state
        .repository
        .update_linked_account_in_tenant(
            &tenant.app_id,
            &tenant.tenant_id,
            &req.linked_account_id,
            update,
        )
        .await
        .map_err(|e| {
            tracing::error!(error = %e, "Failed to update commission config");
            Status::internal("Failed to update commission config")
        })?;

    let account = state
        .repository
        .get_linked_account_in_tenant(&tenant.app_id, &tenant.tenant_id, &req.linked_account_id)
        .await
        .map_err(|e| {
            tracing::error!(error = %e, "Failed to fetch updated linked account");
            Status::internal("Failed to fetch linked account")
        })?
        .ok_or_else(|| Status::not_found("Linked account not found"))?;

    Ok(Response::new(UpdateCommissionConfigResponse {
        linked_account: Some(linked_account_to_proto(account)),
    }))
}

fn linked_account_to_proto(a: models::LinkedAccount) -> LinkedAccount {
    LinkedAccount {
        id: a.id,
        app_id: a.app_id,
        tenant_id: a.tenant_id,
        razorpay_account_id: a.razorpay_account_id,
        name: a.name,
        email: a.email,
        status: linked_account_status_to_proto(&a.status).into(),
        commission: a.commission.map(|c| CommissionConfig {
            commission_type: match c.commission_type {
                models::CommissionType::Flat => CommissionType::Flat.into(),
                models::CommissionType::Percentage => CommissionType::Percentage.into(),
            },
            value: c.value,
        }),
        bank_account: a.bank_account.map(|b| BankAccount {
            account_holder_name: b.account_holder_name,
            account_number: b.account_number,
            ifsc_code: b.ifsc_code,
            account_type: b.account_type,
        }),
        legal_info: a.legal_info.map(|l| LegalInfo {
            legal_business_name: l.legal_business_name,
            business_type: l.business_type,
            pan: l.pan,
            gst: l.gst,
        }),
        created_at: datetime_to_timestamp(a.created_at),
        updated_at: datetime_to_timestamp(a.updated_at),
    }
}

fn linked_account_status_to_proto(status: &models::LinkedAccountStatus) -> LinkedAccountStatus {
    match status {
        models::LinkedAccountStatus::Created => LinkedAccountStatus::Created,
        models::LinkedAccountStatus::UnderReview => LinkedAccountStatus::UnderReview,
        models::LinkedAccountStatus::NeedsClarification => LinkedAccountStatus::NeedsClarification,
        models::LinkedAccountStatus::Activated => LinkedAccountStatus::Activated,
        models::LinkedAccountStatus::Suspended => LinkedAccountStatus::Suspended,
    }
}

fn proto_to_linked_account_status(status: i32) -> Option<models::LinkedAccountStatus> {
    match LinkedAccountStatus::try_from(status) {
        Ok(LinkedAccountStatus::Created) => Some(models::LinkedAccountStatus::Created),
        Ok(LinkedAccountStatus::UnderReview) => Some(models::LinkedAccountStatus::UnderReview),
        Ok(LinkedAccountStatus::NeedsClarification) => {
            Some(models::LinkedAccountStatus::NeedsClarification)
        }
        Ok(LinkedAccountStatus::Activated) => Some(models::LinkedAccountStatus::Activated),
        Ok(LinkedAccountStatus::Suspended) => Some(models::LinkedAccountStatus::Suspended),
        _ => None,
    }
}
