/*
feat(sqlx): add migration 20260820000002_entries
*/

/* NOTE: by default, NOT NULL is not enforced on PRIMARY KEY columns in sqlite due to a
         bug in early versions. */
CREATE TABLE IF NOT EXISTS "entries" (
  id         INTEGER NOT NULL PRIMARY KEY AUTOINCREMENT,
  list_id    INTEGER NOT NULL,
  ordinal    INTEGER NOT NULL,
  name       TEXT    NOT NULL,
  author_id  INTEGER NOT NULL,
  created_at INTEGER NOT NULL DEFAULT (unixepoch()),
  updated_at INTEGER NOT NULL,
  UNIQUE(list_id, name),
  FOREIGN KEY (list_id) REFERENCES "lists"(id) ON DELETE CASCADE
);

CREATE INDEX IF NOT EXISTS entries_list_idx
ON "entries" (list_id, name);
