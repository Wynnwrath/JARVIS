ALTER TABLE permission_preferences RENAME TO permission_preferences_old;
CREATE TABLE permission_preferences (
    tool_name TEXT PRIMARY KEY NOT NULL,
    decision  TEXT NOT NULL CHECK(decision IN ('allow', 'deny'))
);
INSERT INTO permission_preferences (tool_name, decision)
    SELECT tool_name, decision FROM permission_preferences_old WHERE path_pattern IS NULL;
DROP TABLE permission_preferences_old;
