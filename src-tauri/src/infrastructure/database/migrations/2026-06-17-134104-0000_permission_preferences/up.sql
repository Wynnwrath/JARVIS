CREATE TABLE IF NOT EXISTS permission_preferences (
    tool_name TEXT PRIMARY KEY NOT NULL,
    decision TEXT NOT NULL CHECK(decision IN ('allow', 'deny'))
);
