/*
feat!: migrate default db engine to postgres

Add migrations 20260830000000_schema, 20260830000001_revisions, 20260830000002_lists, 20260830000003_entries.
Add features to crate: postgres (default), sqlite.
*/

CREATE TABLE IF NOT EXISTS guild.lists (
	id int8 GENERATED ALWAYS AS IDENTITY(INCREMENT BY 1 MINVALUE 1 MAXVALUE 9223372036854775807 START 1 CACHE 1 NO CYCLE) NOT NULL,
	guild_id int8 NOT NULL,
	channel_id int8 NOT NULL,
	message_id int8 NOT NULL,
	author_id int8 NULL,
	revision int8 NOT NULL,
	created_at int8 DEFAULT EXTRACT(epoch FROM now())::bigint NOT NULL,
	updated_at int8 NOT NULL,
	CONSTRAINT lists_channel_id_key UNIQUE (channel_id),
	CONSTRAINT lists_pkey PRIMARY KEY (id),
	CONSTRAINT lists_revision_fkey FOREIGN KEY (revision) REFERENCES guild.revisions(id)
);
