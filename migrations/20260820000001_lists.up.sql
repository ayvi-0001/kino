/*
feat(sqlx): add migration 20260820000001_lists
*/

/* NOTE: by default, NOT NULL is not enforced on PRIMARY KEY columns in sqlite due to a
         bug in early versions. */
CREATE TABLE IF NOT EXISTS "lists" (
  id         INTEGER NOT NULL PRIMARY KEY AUTOINCREMENT,
  guild_id   INTEGER NOT NULL,
  channel_id INTEGER NOT NULL UNIQUE,
  message_id INTEGER NOT NULL,
  author_id  INTEGER,
  revision   INTEGER NOT NULL,
  created_at INTEGER NOT NULL DEFAULT (unixepoch()),
  updated_at INTEGER NOT NULL,
  FOREIGN KEY (revision) REFERENCES "revisions"(id)
);
