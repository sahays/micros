# Identity Resolution Specification

## Problem

Users are identified by `(tenant_id, email)`. This conflates contact information with identity. The same person across tenants appears as unrelated users. No support for government IDs or KYC verification.

## Concepts

| Term | Definition |
|------|------------|
| **User** | Tenant-scoped account with credentials |
| **Identity** | Global representation of a person across tenants |
| **Attribute** | Verified data point (email, phone, Aadhaar, PAN) |
| **Link** | Association between user and identity |

## Data Model

### identities
- `identity_id` (PK): UUID
- `identity_state`: unverified, verified, suspended
- `created_utc`, `updated_utc`: timestamps

### identity_attributes
- `attribute_id` (PK): UUID
- `identity_id` (FK): references identities
- `attribute_type`: email, phone, aadhaar, pan, name, dob
- `attribute_value`: original value
- `normalized_value`: searchable format (lowercase, E.164, digits only)
- `verification_status`: unverified, pending, verified, expired
- `verification_method`: otp, document, aadhaar_otp, digilocker
- `verified_utc`: timestamp
- `metadata`: JSONB for additional context

**Constraint:** UNIQUE(attribute_type, normalized_value) when verified

### user_identity_links
- `user_id` (FK): references users
- `identity_id` (FK): references identities
- `link_confidence`: 0.00 to 1.00
- `link_method`: manual, kyc, fuzzy_match, email_verified
- `created_utc`: timestamp

**Constraint:** PRIMARY KEY(user_id, identity_id)

## Attribute Normalization

| Type | Input | Normalized |
|------|-------|------------|
| email | Alice@Example.COM | alice@example.com |
| phone | +91 98765 43210 | +919876543210 |
| aadhaar | 1234 5678 9012 | 123456789012 |
| pan | abcde1234f | ABCDE1234F |
| name | Dr. Alice Smith Jr. | alice smith |

## Verification Levels

| Level | Requirements | Capabilities |
|-------|--------------|--------------|
| none | Unlinked user | Basic access |
| basic | Email or phone verified | Standard transactions |
| full_kyc | Aadhaar or PAN verified | High-value transactions, regulated services |

## gRPC Service: IdentityService

### ResolveIdentity
Find or create identity for a user based on provided attributes.

**Input:** user_id, attributes[]
**Output:** identity_id, is_new, match_confidence

**Behavior:**
1. Search existing identities by normalized attributes
2. If exact match on verified attribute → link user to existing identity
3. If fuzzy match → return with confidence < 1.0, flag for review
4. If no match → create new identity, link user

### AddAttribute
Add and optionally verify an attribute for an identity.

**Input:** identity_id, attribute_type, attribute_value, verification_proof
**Output:** attribute_id, verification_status

### VerifyAttribute
Complete verification for a pending attribute.

**Input:** attribute_id, verification_code (OTP) or verification_token (document)
**Output:** success, verification_status

### SearchIdentities
Fuzzy search for identities matching criteria.

**Input:** query (name, partial email, phone prefix), attribute_types[], limit
**Output:** matches[] with identity_id, match_score, matched_attributes

**Use case:** Admin lookup, duplicate detection, fraud investigation

### LinkUserToIdentity
Manually link a user account to an existing identity.

**Input:** user_id, identity_id, link_method, confidence
**Output:** success

**Use case:** Customer support merging accounts

### GetLinkedUsers
Get all user accounts linked to an identity.

**Input:** identity_id
**Output:** users[] with user_id, tenant_id, link_confidence, link_method

### InitiateKyc
Start KYC verification flow.

**Input:** identity_id, kyc_type (aadhaar, pan, digilocker)
**Output:** verification_id, redirect_url or otp_sent

### CompleteKyc
Complete KYC with verification response.

**Input:** verification_id, response_data (OTP, callback token)
**Output:** success, verified_attributes[]

## Context Enhancement

### Token Claims (additions)
- `identity_id`: linked identity UUID (optional)
- `identity_level`: none, basic, full_kyc

### Context Headers (additions)
- `X-Identity-ID`: propagated to downstream services
- `X-Identity-Level`: for authorization decisions

### AuthContextResponse (additions)
- `identity.identity_id`
- `identity.verification_level`
- `identity.verified_attributes[]`
- `identity.linked_tenants[]`

## Workflows

### Registration with Identity Resolution
1. User registers with email in Tenant A
2. System creates user account
3. System searches identities by normalized email
4. No match → create identity, link with confidence 1.0
5. Match found → link user to existing identity

### Cross-Tenant Detection
1. User registers same email in Tenant B
2. System finds existing identity via email attribute
3. Links new user to same identity
4. Both user accounts now share identity_id

### KYC Verification (Aadhaar)
1. User initiates Aadhaar KYC
2. System calls UIDAI API, sends OTP to registered mobile
3. User provides OTP
4. System verifies with UIDAI, receives name, DOB, address
5. System adds verified attributes to identity
6. Identity level upgraded to full_kyc

### Duplicate Detection
1. Admin searches by phone number
2. System returns identities with matching/similar phone
3. Admin reviews linked users across tenants
4. Admin can merge identities or flag as fraud

## Edge Cases

- **1000 users named "Anil Kumar" register:** Each gets separate identity (name is not a unique identifier; resolution uses email/phone/Aadhaar)
- **Same email in two tenants:** Same identity, two user accounts linked to it
- **Same name, different email/phone:** Completely separate identities
- **User changes email:** Old email attribute marked inactive, new email added, identity unchanged
- **Two identities later found to share Aadhaar:** Flag for admin review, merge if confirmed same person
- **User deletes account in Tenant A:** User unlinked from identity; identity persists if other users linked
- **Fraudster creates 50 accounts with unique emails:** 50 separate identities until KYC reveals same Aadhaar, then flagged
- **Married name change:** Add new name attribute, keep old; both searchable, identity unchanged
- **Phone number recycled by carrier:** New user gets new identity; old user's phone attribute expires after verification failure

## Authorization Integration

Services can enforce identity requirements:
- Payment > ₹1,00,000 → requires full_kyc
- Document signing → requires basic verification
- Account deletion → requires identity confirmation

## Privacy Considerations

- Attributes encrypted at rest
- PII access logged to audit_events
- Cross-tenant identity visible only to identity owner and admins
- Attribute deletion propagates to all linked users
- GDPR/DPDP compliant data retention policies

## External Integrations

| Provider | Purpose | Attributes Verified |
|----------|---------|---------------------|
| UIDAI | Aadhaar verification | aadhaar, name, dob, address |
| NSDL | PAN verification | pan, name |
| DigiLocker | Document verification | Multiple government IDs |
| Msg91 | Phone OTP | phone |
| SMTP | Email OTP | email |
