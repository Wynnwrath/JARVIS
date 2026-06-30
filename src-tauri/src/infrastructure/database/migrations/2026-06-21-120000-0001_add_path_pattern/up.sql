-- Add per-directory scoping to permission preferences.
-- path_pattern NULL = global rule (applies to all paths for this tool).
-- Non-NULL = directory-scoped (prefix match against the extracted request path).
ALTER TABLE permission_preferences RENAME TO permission_preferences_old;

CREATE TABLE permission_preferences (
    tool_name    TEXT NOT NULL,
    path_pattern TEXT,
    decision     TEXT NOT NULL CHECK(decision IN ('allow', 'deny')),
    PRIMARY KEY (tool_name, path_pattern)
);
-- SQLite treats NULLs as distinct in PRIMARY KEY; use a UNIQUE index that treats NULLs as equal
-- so at most one global rule per tool can coexist with per-path rules.
CREATE UNIQUE INDEX permission_preferences_global_uniq
    ON permission_preferences (tool_name) WHERE path_pattern IS NULL;

INSERT INTO permission_preferences (tool_name, path_pattern, decision)
    SELECT tool_name, NULL, decision FROM permission_preferences_old;

DROP TABLE permission_preferences_old;
