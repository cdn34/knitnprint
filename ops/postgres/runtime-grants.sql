\if :{?database_name}
\else
\error 'pass database_name with -v database_name=...'
\endif
\if :{?migration_role}
\else
\error 'pass migration_role with -v migration_role=...'
\endif
\if :{?runtime_role}
\else
\error 'pass runtime_role with -v runtime_role=...'
\endif
\if :{?reporting_role}
\else
\error 'pass reporting_role with -v reporting_role=...'
\endif

REVOKE CREATE ON SCHEMA public FROM PUBLIC;
REVOKE ALL ON DATABASE :"database_name" FROM PUBLIC;

GRANT CONNECT ON DATABASE :"database_name" TO :"runtime_role", :"reporting_role";
GRANT USAGE ON SCHEMA public TO :"runtime_role", :"reporting_role";
GRANT SELECT, INSERT, UPDATE, DELETE ON ALL TABLES IN SCHEMA public TO :"runtime_role";
GRANT USAGE, SELECT ON ALL SEQUENCES IN SCHEMA public TO :"runtime_role";
GRANT SELECT ON ALL TABLES IN SCHEMA public TO :"reporting_role";

ALTER DEFAULT PRIVILEGES FOR ROLE :"migration_role" IN SCHEMA public
    GRANT SELECT, INSERT, UPDATE, DELETE ON TABLES TO :"runtime_role";
ALTER DEFAULT PRIVILEGES FOR ROLE :"migration_role" IN SCHEMA public
    GRANT USAGE, SELECT ON SEQUENCES TO :"runtime_role";
ALTER DEFAULT PRIVILEGES FOR ROLE :"migration_role" IN SCHEMA public
    GRANT SELECT ON TABLES TO :"reporting_role";

REVOKE CREATE ON SCHEMA public FROM :"runtime_role", :"reporting_role";
REVOKE CREATE, TEMPORARY ON DATABASE :"database_name" FROM :"runtime_role", :"reporting_role";

-- Role creation and passwords belong in the platform secret/IAM workflow. This
-- script intentionally grants no role-management, schema-DDL, or bypass-RLS rights.
