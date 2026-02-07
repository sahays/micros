//! gRPC handlers for customer operations.

use crate::grpc::capability_check::{capabilities, CapabilityMetadata};
use crate::grpc::helpers::{
    check_razorpay_configured, datetime_to_timestamp, extract_tenant_context,
};
use crate::grpc::proto::*;
use crate::models;
use crate::services::razorpay_customers;
use crate::startup::AppState;
use mongodb::bson::DateTime;
use tonic::{Request, Response, Status};
use uuid::Uuid;

pub async fn create_customer(
    state: &AppState,
    request: Request<CreateCustomerRequest>,
) -> Result<Response<CreateCustomerResponse>, Status> {
    if let Some(metadata) = CapabilityMetadata::try_from_request(&request) {
        state
            .capability_checker
            .require_capability_from_metadata(&metadata, capabilities::PAYMENT_CUSTOMER_CREATE)
            .await?;
    }

    check_razorpay_configured(&state.razorpay)?;
    let tenant = extract_tenant_context(&request)?;
    let req = request.into_inner();

    let user_id = tenant
        .user_id
        .clone()
        .ok_or_else(|| Status::invalid_argument("User ID is required"))?;

    // Check for duplicate
    if let Some(_existing) = state
        .repository
        .get_customer_by_user_in_tenant(&tenant.app_id, &tenant.org_id, &user_id)
        .await
        .map_err(|e| {
            tracing::error!(error = %e, "Failed to check existing customer");
            Status::internal("Failed to check existing customer")
        })?
    {
        return Err(Status::already_exists(
            "Customer already exists for this user",
        ));
    }

    let rz_request = razorpay_customers::CreateCustomerRequest {
        name: req.name.clone(),
        email: req.email.clone(),
        contact: req.phone.clone(),
    };

    let rz_response = state
        .razorpay
        .create_customer(rz_request)
        .await
        .map_err(|e| {
            tracing::error!(error = %e, "Failed to create Razorpay customer");
            Status::internal(format!("Failed to create customer: {}", e))
        })?;

    let now = DateTime::now();
    let customer = models::RazorpayCustomer {
        id: Uuid::new_v4().to_string(),
        app_id: tenant.app_id.clone(),
        org_id: tenant.org_id.clone(),
        user_id,
        razorpay_customer_id: rz_response.id,
        name: req.name,
        email: req.email,
        phone: req.phone,
        created_at: now,
        updated_at: now,
    };

    state
        .repository
        .create_customer(customer.clone())
        .await
        .map_err(|e| {
            tracing::error!(error = %e, "Failed to save customer");
            Status::internal("Failed to save customer")
        })?;

    Ok(Response::new(CreateCustomerResponse {
        customer: Some(customer_to_proto(customer)),
    }))
}

pub async fn get_customer(
    state: &AppState,
    request: Request<GetCustomerRequest>,
) -> Result<Response<GetCustomerResponse>, Status> {
    if let Some(metadata) = CapabilityMetadata::try_from_request(&request) {
        state
            .capability_checker
            .require_capability_from_metadata(&metadata, capabilities::PAYMENT_CUSTOMER_READ)
            .await?;
    }

    let tenant = extract_tenant_context(&request)?;
    let req = request.into_inner();

    let customer = state
        .repository
        .get_customer_in_tenant(&tenant.app_id, &tenant.org_id, &req.customer_id)
        .await
        .map_err(|e| {
            tracing::error!(error = %e, "Failed to fetch customer");
            Status::internal("Failed to fetch customer")
        })?
        .ok_or_else(|| Status::not_found("Customer not found"))?;

    Ok(Response::new(GetCustomerResponse {
        customer: Some(customer_to_proto(customer)),
    }))
}

pub async fn update_customer(
    state: &AppState,
    request: Request<UpdateCustomerRequest>,
) -> Result<Response<UpdateCustomerResponse>, Status> {
    if let Some(metadata) = CapabilityMetadata::try_from_request(&request) {
        state
            .capability_checker
            .require_capability_from_metadata(&metadata, capabilities::PAYMENT_CUSTOMER_UPDATE)
            .await?;
    }

    let tenant = extract_tenant_context(&request)?;
    let req = request.into_inner();

    let mut update = mongodb::bson::doc! { "updated_at": DateTime::now() };
    if let Some(name) = &req.name {
        update.insert("name", name);
    }
    if let Some(email) = &req.email {
        update.insert("email", email);
    }
    if let Some(phone) = &req.phone {
        update.insert("phone", phone);
    }

    state
        .repository
        .update_customer_in_tenant(&tenant.app_id, &tenant.org_id, &req.customer_id, update)
        .await
        .map_err(|e| {
            tracing::error!(error = %e, "Failed to update customer");
            Status::internal("Failed to update customer")
        })?;

    let customer = state
        .repository
        .get_customer_in_tenant(&tenant.app_id, &tenant.org_id, &req.customer_id)
        .await
        .map_err(|e| {
            tracing::error!(error = %e, "Failed to fetch customer");
            Status::internal("Failed to fetch customer")
        })?
        .ok_or_else(|| Status::not_found("Customer not found"))?;

    Ok(Response::new(UpdateCustomerResponse {
        customer: Some(customer_to_proto(customer)),
    }))
}

pub async fn list_customers(
    state: &AppState,
    request: Request<ListCustomersRequest>,
) -> Result<Response<ListCustomersResponse>, Status> {
    if let Some(metadata) = CapabilityMetadata::try_from_request(&request) {
        state
            .capability_checker
            .require_capability_from_metadata(&metadata, capabilities::PAYMENT_CUSTOMER_READ)
            .await?;
    }

    let tenant = extract_tenant_context(&request)?;
    let req = request.into_inner();

    let limit = req.limit.clamp(1, 100) as i64;
    let offset = req.offset.max(0) as u64;

    let (customers, total_count) = state
        .repository
        .list_customers_in_tenant(&tenant.app_id, &tenant.org_id, limit, offset)
        .await
        .map_err(|e| {
            tracing::error!(error = %e, "Failed to list customers");
            Status::internal("Failed to list customers")
        })?;

    Ok(Response::new(ListCustomersResponse {
        customers: customers.into_iter().map(customer_to_proto).collect(),
        total_count,
    }))
}

fn customer_to_proto(c: models::RazorpayCustomer) -> RazorpayCustomer {
    RazorpayCustomer {
        id: c.id,
        app_id: c.app_id,
        org_id: c.org_id,
        user_id: c.user_id,
        razorpay_customer_id: c.razorpay_customer_id,
        name: c.name,
        email: c.email,
        phone: c.phone,
        created_at: datetime_to_timestamp(c.created_at),
        updated_at: datetime_to_timestamp(c.updated_at),
    }
}
