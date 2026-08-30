/*
feat(sqlx): add migration 20260820000000_revisions
*/

/* NOTE: by default, NOT NULL is not enforced on PRIMARY KEY columns in sqlite due to a
         bug in early versions. */
CREATE TABLE IF NOT EXISTS "revisions" (
  id         INTEGER NOT NULL PRIMARY KEY AUTOINCREMENT,
  guild_id   INTEGER NOT NULL,
  channel_id INTEGER NOT NULL,
  author_id  INTEGER NOT NULL,
  content    TEXT    NOT NULL DEFAULT '',
  created_at INTEGER NOT NULL DEFAULT (unixepoch())
);

CREATE INDEX IF NOT EXISTS revisions_guild_idx
ON "revisions" (guild_id, id DESC);
