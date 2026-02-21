-- Add academy::* wildcard capability for academy admins.
-- This grants full access to academy management features (venues, sports,
-- packages, coaches, members, staff, banking, documents, settings).

INSERT INTO capabilities (cap_key)
VALUES ('academy::*')
ON CONFLICT (cap_key) DO NOTHING;

-- Assign academy::* to every existing "Admin" role so that all academy admins
-- get the capability automatically.
INSERT INTO role_capabilities (role_id, cap_id)
SELECT r.role_id, c.cap_id
FROM roles r
CROSS JOIN capabilities c
WHERE r.role_label = 'Admin'
  AND c.cap_key = 'academy::*'
ON CONFLICT DO NOTHING;
