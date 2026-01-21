# Story: Identity Context

- [ ] **Status: Planning**
- **Epic:** [002-identity-resolution](../epics/002-identity-resolution.md)
- **Priority:** P1

## Summary

Add identity information to JWT claims and context headers for downstream service authorization.

## Tasks

- [ ] Add identity_id, identity_level to AccessTokenClaims
- [ ] Update token generation to include identity context
- [ ] Update token validation to extract identity context
- [ ] Add identity to AuthContextResponse
- [ ] Document new context headers for BFF

## Token Claims (additions)

| Claim | Type | Description |
|-------|------|-------------|
| identity_id | String (UUID) | Linked identity, null if none |
| identity_level | String | none, basic, full_kyc |

## Context Headers (additions)

| Header | Description |
|--------|-------------|
| X-Identity-ID | Identity UUID |
| X-Identity-Level | Verification level |

## AuthContextResponse (additions)

| Field | Type | Description |
|-------|------|-------------|
| identity | IdentityContext | null if no linked identity |

### IdentityContext
| Field | Type |
|-------|------|
| identity_id | UUID |
| verification_level | String |
| verified_attributes | String[] |
| linked_tenant_count | i32 |

## Migration Path

1. Tokens without identity claims treated as identity_level = 'none'
2. Existing users get identity linked on next login
3. Downstream services handle missing X-Identity-ID gracefully

## Acceptance Criteria

- [ ] New tokens include identity_id if user has linked identity
- [ ] New tokens include identity_level
- [ ] AuthContextResponse includes identity details
- [ ] BFF propagates X-Identity-ID header
- [ ] BFF propagates X-Identity-Level header
- [ ] Services without identity support continue working
- [ ] Token validation backward compatible
