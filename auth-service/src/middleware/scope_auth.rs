use crate::{middleware::ServiceContext, models::AuditLog, AppState};
use service_core::{
    axum::{
        extract::{Request, State},
        middleware::Next,
        response::Response,
    },
    error::AppError,
};

/// Middleware to require specific scopes for service authentication
pub async fn require_scopes(
    State(state): State<AppState>,
    required_scopes: Vec<String>,
    req: Request,
    next: Next,
) -> Result<Response, AppError> {
    let method = req.method().to_string();
    let endpoint = req.uri().path().to_string();
    let ip_address = req
        .headers()
        .get("x-forwarded-for")
        .and_then(|v| v.to_str().ok())
        .unwrap_or("unknown")
        .to_string();

    let context = req.extensions().get::<ServiceContext>().ok_or_else(|| {
        AppError::InternalError(anyhow::anyhow!(
            "Service context missing from request extensions"
        ))
    })?;

    for required in &required_scopes {
        if !has_scope(&context.scopes, required) {
            tracing::warn!(
                service_id = %context.service_id,
                required_scope = %required,
                granted_scopes = ?context.scopes,
                "Insufficient scopes"
            );

            // Audit log failure
            let mut log = AuditLog::new(
                "access_denied".to_string(),
                Some(context.service_id.clone()),
                endpoint.clone(),
                method.clone(),
                service_core::axum::http::StatusCode::FORBIDDEN.as_u16(),
                ip_address.clone(),
            );
            log.service_name = Some(context.service_name.clone());
            log.details = Some(format!("Insufficient scopes. Required: {}", required));
            log.scopes = Some(context.scopes.clone());
            let db = state.db.clone();
            tokio::spawn(async move {
                let _ = db.audit_logs().insert_one(log, None).await;
            });

            return Err(AppError::Forbidden(anyhow::anyhow!(
                "Insufficient scopes. Required: {}",
                required
            )));
        }
    }

    Ok(next.run(req).await)
}

fn has_scope(granted_scopes: &[String], required: &str) -> bool {
    for granted in granted_scopes {
        if granted == "*" {
            return true;
        }
        if granted == required {
            return true;
        }
        if let Some(prefix) = granted.strip_suffix('*') {
            if required.starts_with(prefix) {
                return true;
            }
        }
    }
    false
}
