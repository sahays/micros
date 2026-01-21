# Story: Fuzzy Search

- [ ] **Status: Planning**
- **Epic:** [002-identity-resolution](../epics/002-identity-resolution.md)
- **Priority:** P2

## Summary

Implement trigram-based fuzzy search for identities to support duplicate detection and admin lookups.

## Tasks

- [ ] Implement `SearchIdentities` RPC
- [ ] Build trigram similarity query
- [ ] Support multiple attribute type filters
- [ ] Return ranked results with match scores
- [ ] Add pagination support
- [ ] Optimize query performance

## gRPC Method

### SearchIdentities
**Input:**
- query: Search string
- attribute_types: Filter by types (name, email, phone)
- min_score: Minimum similarity threshold (0.0-1.0)
- limit: Max results
- offset: Pagination

**Output:**
- matches[]: identity_id, match_score, matched_attribute, matched_value

## Search Modes

| Mode | Use Case | Threshold |
|------|----------|-----------|
| Exact prefix | Phone lookup | N/A (LIKE query) |
| Trigram similarity | Name search | 0.3 default |
| Exact normalized | Email lookup | N/A (= query) |

## Query Strategy

1. **Phone/Email:** Exact or prefix match on normalized_value
2. **Name:** Trigram similarity using pg_trgm `%` operator
3. **Combined:** OR across attribute types, ranked by score

## Performance

- Trigram GIN index on normalized_value
- Limit results to prevent full table scan
- Cache frequent searches (optional)

## Acceptance Criteria

- [ ] Search by partial name returns fuzzy matches
- [ ] Search by phone prefix returns exact matches
- [ ] Search by email returns exact match
- [ ] Results ranked by similarity score
- [ ] Minimum score threshold filters low matches
- [ ] Pagination works correctly
- [ ] Query completes under 100ms for 1M identities
- [ ] Service permission required for search
