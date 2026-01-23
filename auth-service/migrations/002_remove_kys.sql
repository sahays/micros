-- Remove Know-Your-Service (KYS) tables
-- Internal services have unrestricted access, no need for service registry

-- Drop tables in dependency order
DROP TABLE IF EXISTS service_sessions;
DROP TABLE IF EXISTS service_permissions;
DROP TABLE IF EXISTS service_secrets;
DROP TABLE IF EXISTS services;
