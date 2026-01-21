# Epic: Identity Resolution & KYC

Status: planning
Created: 2026-01-21

## Overview

Introduce global identity layer separating "person" from "user account". Enables cross-tenant identity linking, KYC verification, and fuzzy matching for fraud detection.

## Problem

- Users identified by `(tenant_id, email)` conflates contact with identity
- Same person across tenants appears as unrelated users
- No KYC/government ID support
- No duplicate detection or fraud prevention

## Solution

- `identities` table for global person representation
- `identity_attributes` for verified data points (email, phone, Aadhaar, PAN)
- `user_identity_links` connecting user accounts to identities
- gRPC IdentityService for resolution, verification, and search

## Database Schema

### New Tables
- `identities` - Global identity records
- `identity_attributes` - Normalized, verified attributes
- `user_identity_links` - User-to-identity associations

### Schema Changes
- Add `identity_id`, `identity_level` to JWT claims
- Add `X-Identity-ID`, `X-Identity-Level` context headers

## Stories

### Phase 1: Foundation

- [ ] [015-identity-schema](../stories/015-identity-schema.md) - Database tables and migrations
- [ ] [016-identity-service](../stories/016-identity-service.md) - Core gRPC service and resolution logic
- [ ] [017-attribute-normalization](../stories/017-attribute-normalization.md) - Normalization and indexing

### Phase 2: Verification

- [ ] [018-email-phone-verification](../stories/018-email-phone-verification.md) - OTP-based attribute verification
- [ ] [019-aadhaar-kyc](../stories/019-aadhaar-kyc.md) - UIDAI integration
- [ ] [020-pan-verification](../stories/020-pan-verification.md) - NSDL integration

### Phase 3: Context Integration

- [ ] [021-identity-context](../stories/021-identity-context.md) - Token claims and header propagation
- [ ] [022-cross-tenant-linking](../stories/022-cross-tenant-linking.md) - Auto-link on registration

### Phase 4: Search & Admin

- [ ] [023-fuzzy-search](../stories/023-fuzzy-search.md) - Trigram-based identity search
- [ ] [024-admin-tools](../stories/024-admin-tools.md) - Merge, split, flag identities

## gRPC Methods

### IdentityService
- `ResolveIdentity` - Find or create identity for user
- `AddAttribute` - Add attribute to identity
- `VerifyAttribute` - Complete attribute verification
- `SearchIdentities` - Fuzzy search
- `LinkUserToIdentity` - Manual linking
- `GetLinkedUsers` - Get all users for identity
- `InitiateKyc` - Start KYC flow
- `CompleteKyc` - Finish KYC verification

## Acceptance Criteria

- [ ] Identity tables created with proper constraints
- [ ] Attributes normalized consistently across types
- [ ] Email/phone verification via existing OTP infrastructure
- [ ] Aadhaar verification via UIDAI API
- [ ] PAN verification via NSDL API
- [ ] Identity context in JWT claims
- [ ] Context headers propagated to downstream services
- [ ] Fuzzy search returns ranked matches
- [ ] Cross-tenant users auto-linked by verified attributes
- [ ] Admin can merge/split identities
- [ ] All mutations logged to audit_events
