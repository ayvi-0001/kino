/*
feat!: migrate default db engine to postgres

Add migrations 20260830000000_schema, 20260830000001_revisions, 20260830000002_lists, 20260830000003_entries.
Add features to crate: postgres (default), sqlite.
*/

CREATE TABLE IF NOT EXISTS guild.entries (
	id int8 GENERATED ALWAYS AS IDENTITY(INCREMENT BY 1 MINVALUE 1 MAXVALUE 9223372036854775807 START 1 CACHE 1 NO CYCLE) NOT NULL,
	list_id int8 NOT NULL,
	ordinal int8 NOT NULL,
	"name" text NOT NULL,
	author_id int8 NOT NULL,
	created_at int8 DEFAULT EXTRACT(epoch FROM now())::bigint NOT NULL,
	updated_at int8 NOT NULL,
	CONSTRAINT entries_list_id_name_key UNIQUE (list_id, name),
	CONSTRAINT entries_pkey PRIMARY KEY (id),
	CONSTRAINT entries_list_id_fkey FOREIGN KEY (list_id) REFERENCES guild.lists(id) ON DELETE CASCADE
);

CREATE INDEX IF NOT EXISTS entries_list_idx ON guild.entries USING btree (list_id, name);
