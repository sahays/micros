# Story: AI Matching Suggestions (Optional)

- [ ] **Status: Planning**
- **Epic:** [001-reconciliation-service](../epics/001-reconciliation-service.md)
- **Priority:** Low (Nice-to-have, not core functionality)

## Summary

Implement GetAiSuggestions, ConfirmSuggestion, and RejectSuggestion gRPC methods for **optional** AI-assisted matching suggestions.

**Important**: This is **supplementary** functionality. The core matching workflow is:
1. **Auto-matching via rules** (Story 004) - Primary
2. **Manual matching by user** (Story 005) - Primary
3. **AI suggestions** (This story) - Optional assistance for difficult matches

## Core Matching vs AI Suggestions

```
┌─────────────────────────────────────────────────────────────┐
│  MATCHING WORKFLOW                                          │
├─────────────────────────────────────────────────────────────┤
│                                                             │
│  1. AUTO-MATCH (Story 004) - PRIMARY                        │
│     Rule engine applies matching rules to transactions      │
│     → Pattern matching on description                       │
│     → Amount/date matching to ledger entries                │
│     → Runs automatically on statement commit                │
│                                                             │
│  2. MANUAL MATCH (Story 005) - PRIMARY                      │
│     User reviews unmatched transactions                     │
│     → Match: Link to ledger entry                           │
│     → Exclude: Mark as not needing match                    │
│     → Skip: Leave unmatched for later                       │
│                                                             │
│  3. AI SUGGESTIONS (This Story) - OPTIONAL                  │
│     For remaining difficult matches, user can request       │
│     AI suggestions as hints                                 │
│     → User must still confirm/reject each suggestion        │
│     → AI never auto-matches                                 │
│                                                             │
└─────────────────────────────────────────────────────────────┘
```

## When to Use AI Suggestions

AI suggestions are useful when:
- Many unmatched transactions remain after rule-based matching
- Descriptions are complex or inconsistent
- User wants hints for difficult matches

AI suggestions are **NOT**:
- A replacement for rules or manual matching
- Auto-applied (always require user confirmation)
- Required for reconciliation to work

## Tasks

- [ ] Define proto messages: AiSuggestion, GetAiSuggestionsRequest/Response
- [ ] Define proto messages: ConfirmSuggestionRequest/Response, RejectSuggestionRequest/Response
- [ ] Implement GetAiSuggestions handler
- [ ] Integrate with genai-service for suggestion generation
- [ ] Implement ConfirmSuggestion handler
- [ ] Implement RejectSuggestion handler
- [ ] Store suggestion feedback for future improvements
- [ ] Add capability checks to all methods

## gRPC Methods

### GetAiSuggestions
**Input:** tenant_id (from auth), reconciliation_id, limit (optional), min_confidence (optional)
**Output:** suggestions[] with confidence scores and explanations

**Behavior:**
1. Gather unmatched bank transactions for the reconciliation
2. Query candidate ledger entries (by date range, bank's ledger account)
3. Send to genai-service requesting match suggestions
4. Parse response for suggested matches with confidence scores
5. Filter by min_confidence if specified
6. Store suggestions in ai_suggestions table (status: pending)
7. Return suggestions ordered by confidence (highest first)

**Note:** This is a user-initiated action, not automatic. User explicitly requests suggestions.

**Capability:** `reconciliation.ai_suggestion:read`

### ConfirmSuggestion
**Input:** tenant_id (from auth), suggestion_id
**Output:** match (TransactionMatch)

**Behavior:**
1. Validate suggestion exists and is pending
2. Create match with method = "ai_confirmed"
3. Update bank transaction status to "matched"
4. Mark suggestion as confirmed
5. Store as positive feedback
6. Return the created match

**Capability:** `reconciliation.ai_suggestion:confirm`

### RejectSuggestion
**Input:** tenant_id (from auth), suggestion_id, reason (optional)
**Output:** success

**Behavior:**
1. Validate suggestion exists and is pending
2. Mark suggestion as rejected
3. Store rejection reason as feedback
4. Do not create match
5. Transaction remains unmatched

**Capability:** `reconciliation.ai_suggestion:reject`

## GenAI Integration

### Request to genai-service

```json
{
  "prompt_type": "transaction_matching_suggestions",
  "context": {
    "unmatched_transactions": [
      {
        "id": "uuid",
        "date": "2026-01-15",
        "description": "NEFT-ACME CORP-REF123",
        "amount": "-15000.00"
      }
    ],
    "candidate_ledger_entries": [
      {
        "id": "uuid",
        "date": "2026-01-14",
        "description": "Payment to ACME Corporation",
        "amount": "-15000.00"
      }
    ]
  },
  "output_format": "structured_json"
}
```

### Response from genai-service

```json
{
  "suggestions": [
    {
      "bank_transaction_id": "uuid",
      "ledger_entry_id": "uuid",
      "confidence": 0.85,
      "explanation": "Amount matches exactly (-15000.00). Description contains 'ACME CORP' similar to ledger entry 'ACME Corporation'. Date within 1 day."
    }
  ]
}
```

## Confidence Scores

Confidence scores help user prioritize which suggestions to review:

- **High (>0.8)**: Strong match indicators, review first
- **Medium (0.5-0.8)**: Possible match, needs user judgment
- **Low (<0.5)**: Weak match, likely not correct

**Important:** All suggestions require user confirmation regardless of confidence. There is no auto-accept.

## Suggestion Feedback

Store feedback to potentially improve future suggestions:
- **Confirmed**: User accepted the suggestion
- **Rejected**: User rejected (with optional reason)

This feedback can be used to refine prompts or train models in the future, but is not required for the core functionality.

## Database Schema

Uses existing `ai_suggestions` table:

```sql
CREATE TABLE ai_suggestions (
    suggestion_id UUID PRIMARY KEY,
    tenant_id UUID NOT NULL,
    bank_transaction_id UUID NOT NULL,
    ledger_entry_id UUID NOT NULL,
    confidence_score DOUBLE PRECISION NOT NULL,
    explanation TEXT,
    status VARCHAR(20) DEFAULT 'pending',  -- pending, confirmed, rejected
    rejection_reason TEXT,
    created_utc TIMESTAMPTZ DEFAULT NOW(),
    resolved_utc TIMESTAMPTZ
);
```

## Acceptance Criteria

- [ ] GetAiSuggestions returns suggestions with confidence scores
- [ ] GetAiSuggestions is user-initiated (not automatic)
- [ ] Suggestions include explanation text
- [ ] ConfirmSuggestion creates match and updates transaction status
- [ ] RejectSuggestion marks suggestion as rejected
- [ ] All suggestions require user confirmation (no auto-accept)
- [ ] Reconciliation works without any AI suggestions
- [ ] genai-service failures handled gracefully (return empty list)
- [ ] All methods enforce tenant isolation
- [ ] All methods check capabilities

## Integration Tests

- [ ] Get suggestions for unmatched transactions returns results
- [ ] Get suggestions with min_confidence filters correctly
- [ ] Confirm suggestion creates match
- [ ] Reject suggestion stores feedback
- [ ] Reconciliation completes without using AI suggestions
- [ ] Service works normally if genai-service unavailable
- [ ] Operations without capability return PERMISSION_DENIED
