# Story: Identity Schema

- [ ] **Status: Planning**
- **Epic:** [002-identity-resolution](../epics/002-identity-resolution.md)
- **Priority:** P0

## Summary

Create database tables for identity resolution: identities, identity_attributes, user_identity_links.

## Tasks

- [ ] Create migration for `identities` table
- [ ] Create migration for `identity_attributes` table
- [ ] Create migration for `user_identity_links` table
- [ ] Add indexes for attribute lookups
- [ ] Install pg_trgm extension for fuzzy matching
- [ ] Create trigram index on normalized_value
- [ ] Add sqlx models for new tables

## Schema

### identities
| Column | Type | Constraints |
|--------|------|-------------|
| identity_id | UUID | PK |
| identity_state | VARCHAR(20) | NOT NULL, DEFAULT 'unverified' |
| created_utc | TIMESTAMPTZ | NOT NULL |
| updated_utc | TIMESTAMPTZ | NOT NULL |

### identity_attributes
| Column | Type | Constraints |
|--------|------|-------------|
| attribute_id | UUID | PK |
| identity_id | UUID | FK → identities |
| attribute_type | VARCHAR(50) | NOT NULL |
| attribute_value | TEXT | NOT NULL |
| normalized_value | TEXT | NOT NULL |
| verification_status | VARCHAR(20) | DEFAULT 'unverified' |
| verification_method | VARCHAR(50) | NULL |
| verified_utc | TIMESTAMPTZ | NULL |
| metadata | JSONB | NULL |
| created_utc | TIMESTAMPTZ | NOT NULL |

**Constraints:**
- UNIQUE(attribute_type, normalized_value) WHERE verification_status = 'verified'

### user_identity_links
| Column | Type | Constraints |
|--------|------|-------------|
| user_id | UUID | FK → users |
| identity_id | UUID | FK → identities |
| link_confidence | DECIMAL(3,2) | NOT NULL |
| link_method | VARCHAR(50) | NOT NULL |
| created_utc | TIMESTAMPTZ | NOT NULL |

**Constraints:**
- PK(user_id, identity_id)

## Acceptance Criteria

- [ ] Migrations run successfully
- [ ] pg_trgm extension enabled
- [ ] Trigram index created on normalized_value for name/display_name
- [ ] Foreign key constraints enforce referential integrity
- [ ] Unique constraint prevents duplicate verified attributes
- [ ] sqlx compile-time checks pass
