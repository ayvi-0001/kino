/*
feat!: migrate default db engine to postgres

Add migrations 20260830000000_schema, 20260830000001_revisions, 20260830000002_lists, 20260830000003_entries.
Add features to crate: postgres (default), sqlite.
*/

CREATE TABLE IF NOT EXISTS guild.revisions (
	id int8 GENERATED ALWAYS AS IDENTITY(INCREMENT BY 1 MINVALUE 1 MAXVALUE 9223372036854775807 START 1 CACHE 1 NO CYCLE) NOT NULL,
	guild_id int8 NOT NULL,
	channel_id int8 NOT NULL,
	author_id int8 NOT NULL,
	"content" text DEFAULT ''::text NOT NULL,
	created_at int8 DEFAULT EXTRACT(epoch FROM now())::bigint NOT NULL,
	CONSTRAINT revisions_pkey PRIMARY KEY (id)
);

CREATE INDEX IF NOT EXISTS revisions_guild_idx ON guild.revisions USING btree (guild_id, id DESC);
