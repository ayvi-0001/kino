/*
feat!: migrate default db engine to postgres

Add migrations 20260830000000_schema, 20260830000001_revisions, 20260830000002_lists, 20260830000003_entries.
Add features to crate: postgres (default), sqlite.
*/

CREATE SCHEMA IF NOT EXISTS guild;
