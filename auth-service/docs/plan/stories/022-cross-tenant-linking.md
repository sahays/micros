# Story: Cross-Tenant Linking

- [ ] **Status: Planning**
- **Epic:** [002-identity-resolution](../epics/002-identity-resolution.md)
- **Priority:** P1

## Summary

Automatically link users to existing identities during registration when verified attributes match.

## Tasks

- [ ] Hook identity resolution into registration flow
- [ ] Hook identity resolution into login flow (first login after feature)
- [ ] Search by email on registration
- [ ] Link to existing identity if found
- [ ] Create new identity if not found
- [ ] Handle confidence thresholds

## Registration Flow (updated)

1. User registers with email in Tenant B
2. Normalize email
3. Search identity_attributes for verified email match
4. **Match found:** Link user to existing identity (confidence 1.0)
5. **No match:** Create new identity, add unverified email attribute, link user

## Login Flow (migration)

For users registered before identity resolution:
1. User logs in
2. Check if user has linked identity
3. **No link:** Run ResolveIdentity with user's email
4. Link user to found/created identity
5. Include identity in new token

## Linking Rules

| Scenario | Action | Confidence |
|----------|--------|------------|
| Exact email match (verified) | Auto-link | 1.0 |
| Exact phone match (verified) | Auto-link | 1.0 |
| Exact Aadhaar match | Auto-link | 1.0 |
| Name fuzzy match only | Flag for review | 0.5-0.9 |
| No match | Create new identity | 1.0 |

## Acceptance Criteria

- [ ] New registrations check for existing identity
- [ ] Existing users linked on next login
- [ ] Same email across tenants shares identity
- [ ] Same phone across tenants shares identity
- [ ] Fuzzy matches flagged, not auto-linked
- [ ] User informed of linked tenants (optional, privacy setting)
- [ ] Audit event on identity link
