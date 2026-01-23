# Story: AI Matching

- [ ] **Status: Planning**
- **Epic:** [001-reconciliation-service](../epics/001-reconciliation-service.md)

## Summary

Implement GetAiSuggestions, ConfirmSuggestion, and RejectSuggestion gRPC methods for AI-powered transaction matching via genai-service.

## Tasks

- [ ] Define proto messages: AiSuggestion, GetAiSuggestionsRequest/Response
- [ ] Define proto messages: ConfirmSuggestionRequest/Response, RejectSuggestionRequest/Response
- [ ] Implement GetAiSuggestions handler
- [ ] Integrate with genai-service for matching
- [ ] Implement confidence scoring
- [ ] Implement ConfirmSuggestion handler
- [ ] Implement RejectSuggestion handler
- [ ] Store feedback for learning

## gRPC Methods

### GetAiSuggestions
**Input:** tenant_id, bank_transaction_ids[] (or statement_id for all unmatched)
**Output:** suggestions[] with confidence scores

**Behavior:**
1. Gather unmatched bank transactions
2. Query candidate ledger entries (by date range, account)
3. Send to genai-service with context:
   - Bank transaction details
   - Candidate ledger entries
   - Historical matching patterns for tenant
4. Parse AI response for suggested matches
5. Return suggestions with confidence scores

### ConfirmSuggestion
**Input:** tenant_id, suggestion_id
**Output:** match

**Behavior:**
- Create match with type = ai_confirmed
- Store as positive feedback for learning

### RejectSuggestion
**Input:** tenant_id, suggestion_id, reason (optional)
**Output:** success

**Behavior:**
- Mark suggestion as rejected
- Store as negative feedback for learning
- Do not create match

## GenAI Integration

### Request to genai-service
- Prompt with bank transaction details
- List of candidate ledger entries
- Historical patterns (common merchants, recurring transactions)
- Request: Match suggestions with confidence and explanation

### Response parsing
- Extract suggested matches
- Extract confidence score (0-1)
- Extract explanation for user

### Confidence Thresholds
- High (>0.9): Could auto-accept with tenant config
- Medium (0.7-0.9): Suggest prominently
- Low (<0.7): Show but flag as uncertain

## Learning

Store feedback to improve future suggestions:
- Confirmed matches: Positive signal
- Rejected matches: Negative signal
- Manual matches: Pattern to learn

Feed historical patterns back into prompts for better accuracy over time.

## Acceptance Criteria

- [ ] GetAiSuggestions returns suggestions with scores
- [ ] Suggestions include explanation text
- [ ] ConfirmSuggestion creates match
- [ ] RejectSuggestion stores negative feedback
- [ ] High-confidence suggestions ranked first
- [ ] Historical patterns influence suggestions
- [ ] genai-service failures handled gracefully

## Integration Tests

- [ ] Get suggestions for unmatched transactions
- [ ] Confirm suggestion creates ai_confirmed match
- [ ] Reject suggestion stores feedback
- [ ] Suggestions improve with confirmed matches
- [ ] Service degrades gracefully if genai unavailable
