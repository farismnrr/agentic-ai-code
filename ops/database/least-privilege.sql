\set ON_ERROR_STOP on

-- Run with an authorized database owner/admin after creating LOGIN roles and
-- assigning passwords outside this repository. Required psql variables:
--   runtime_role   application LOGIN role
--   migration_role schema-owner/migration LOGIN role
-- Credentials are deliberately never accepted by this file.

SELECT :'runtime_role' <> ''
   AND :'migration_role' <> ''
   AND :'runtime_role' <> :'migration_role' AS role_names_valid
\gset
\if :role_names_valid
\else
  \echo 'runtime_role and migration_role must be distinct non-empty roles'
  \quit 3
\endif

SELECT EXISTS (SELECT 1 FROM pg_roles WHERE rolname = :'runtime_role') AS runtime_role_exists,
       EXISTS (SELECT 1 FROM pg_roles WHERE rolname = :'migration_role') AS migration_role_exists
\gset
\if :runtime_role_exists
\else
  \echo 'runtime role does not exist'
  \quit 3
\endif
\if :migration_role_exists
\else
  \echo 'migration role does not exist'
  \quit 3
\endif

-- Runtime role: data access only. Revoke broad object authority first so the
-- resulting contract is deterministic even if the role previously had grants.
REVOKE ALL ON SCHEMA ai_code FROM :"runtime_role";
REVOKE ALL ON ALL TABLES IN SCHEMA ai_code FROM :"runtime_role";
REVOKE ALL ON ALL SEQUENCES IN SCHEMA ai_code FROM :"runtime_role";
GRANT CONNECT ON DATABASE :"DBNAME" TO :"runtime_role";
GRANT USAGE ON SCHEMA ai_code TO :"runtime_role";
GRANT SELECT, INSERT, UPDATE, DELETE ON ALL TABLES IN SCHEMA ai_code TO :"runtime_role";
GRANT USAGE, SELECT ON ALL SEQUENCES IN SCHEMA ai_code TO :"runtime_role";

-- Future objects created by the migration owner inherit only runtime DML grants.
ALTER DEFAULT PRIVILEGES FOR ROLE :"migration_role" IN SCHEMA ai_code
  GRANT SELECT, INSERT, UPDATE, DELETE ON TABLES TO :"runtime_role";
ALTER DEFAULT PRIVILEGES FOR ROLE :"migration_role" IN SCHEMA ai_code
  GRANT USAGE, SELECT ON SEQUENCES TO :"runtime_role";

SELECT NOT (rolsuper OR rolcreaterole OR rolcreatedb OR rolreplication OR rolbypassrls) AS runtime_role_safe
  FROM pg_roles
 WHERE rolname = :'runtime_role'
\gset
\if :runtime_role_safe
\else
  \echo 'runtime role has forbidden PostgreSQL role attributes'
  \quit 3
\endif
