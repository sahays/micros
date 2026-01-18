# Story: Tenant & User Data Model

- [x] **Status: Completed**
- **Epic:** [001-auth-service-v2](../epics/001-auth-service-v2.md)
- **Priority:** P0

## Summary

Implement tenant and user models with proper identity management.

## Tasks

- [x] Create `models/tenant.rs` - Tenant struct and queries
- [x] Create `models/user.rs` - User struct and queries
- [x] Create `models/user_identity.rs` - Identity providers
- [x] Create tenant repository with CRUD
- [x] Create user repository with CRUD
- [x] Add tenant admin endpoints (create, get, list)
- [x] Add user admin endpoints (create, get, update state)

## Models

```rust
// Tenant
pub struct Tenant {
    pub tenant_id: Uuid,
    pub tenant_label: String,
    pub tenant_state_code: TenantState, // active, suspended
    pub created_utc: DateTime<Utc>,
}

// User
pub struct User {
    pub user_id: Uuid,
    pub tenant_id: Uuid,
    pub email_addr: Option<String>,
    pub phone_e164: Option<String>,
    pub display_label: Option<String>,
    pub user_state_code: UserState, // active, inactive
    pub created_utc: DateTime<Utc>,
    pub updated_utc: DateTime<Utc>,
}

// UserIdentity
pub struct UserIdentity {
    pub ident_id: Uuid,
    pub user_id: Uuid,
    pub ident_provider_code: IdentProvider, // password, google
    pub provider_subject_key: Option<String>,
    pub email_verified_flag: bool,
    pub phone_verified_flag: bool,
    pub password_hash_text: Option<String>,
    pub created_utc: DateTime<Utc>,
}
```

## API Endpoints

```
POST   /admin/tenants           - Create tenant
GET    /admin/tenants           - List tenants
GET    /admin/tenants/{id}      - Get tenant
PATCH  /admin/tenants/{id}      - Update tenant state

POST   /admin/users             - Create user (admin)
GET    /admin/users/{id}        - Get user
PATCH  /admin/users/{id}/state  - Update user state
```

## Acceptance Criteria

- [x] Tenant CRUD working
- [x] User CRUD working with tenant scoping
- [x] Email uniqueness per tenant enforced
- [x] Phone uniqueness per tenant enforced
- [x] User identity linking works
