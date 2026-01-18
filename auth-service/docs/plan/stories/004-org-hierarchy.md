# Story: Org Node Hierarchy

Status: completed
Epic: [001-auth-service-v2](../epics/001-auth-service-v2.md)
Priority: P0

## Summary

Implement org node tree with closure table for efficient subtree queries.

## Tasks

- [x] Create `models/org_node.rs` - OrgNode struct
- [x] Create `models/org_node_path.rs` - Closure table
- [x] Implement closure table maintenance (insert/delete)
- [x] Create org node repository
- [x] Add tree query helpers (ancestors, descendants, subtree)
- [x] Create org node API endpoints

## Closure Table Operations

```sql
-- On node insert: add self-reference + copy parent paths
INSERT INTO org_node_paths (tenant_id, ancestor_org_node_id, descendant_org_node_id, depth_val)
SELECT tenant_id, ancestor_org_node_id, NEW.org_node_id, depth_val + 1
FROM org_node_paths
WHERE descendant_org_node_id = NEW.parent_org_node_id
UNION ALL
SELECT NEW.tenant_id, NEW.org_node_id, NEW.org_node_id, 0;

-- Get all descendants of a node
SELECT descendant_org_node_id FROM org_node_paths
WHERE ancestor_org_node_id = $1;

-- Get all ancestors of a node
SELECT ancestor_org_node_id FROM org_node_paths
WHERE descendant_org_node_id = $1
ORDER BY depth_val DESC;

-- Check if node A is ancestor of node B
SELECT EXISTS(
  SELECT 1 FROM org_node_paths
  WHERE ancestor_org_node_id = $1 AND descendant_org_node_id = $2
);
```

## API Endpoints

```
POST /org/nodes
{
  "tenant_id": "uuid",
  "node_type_code": "region",
  "node_label": "North Region",
  "parent_org_node_id": "uuid" | null
}

PATCH /org/nodes/{org_node_id}
{ "node_label": "Updated Name" }

POST /org/nodes/{org_node_id}/deactivate

GET /org/nodes/{org_node_id}

GET /org/nodes/{org_node_id}/children

GET /org/tree?tenant_id=uuid
Response: Nested tree structure
```

## Acceptance Criteria

- [x] Nodes created with closure table entries
- [x] Subtree queries work efficiently
- [x] Ancestor queries work
- [x] Tree endpoint returns full hierarchy
- [x] Deactivate marks node inactive (no delete)
