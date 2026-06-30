CREATE TABLE IF NOT EXISTS session_messages (
    session_id  TEXT    NOT NULL,
    seq         INTEGER NOT NULL,
    role        TEXT    NOT NULL,
    content_json TEXT   NOT NULL,
    created_at  BIGINT  NOT NULL,
    PRIMARY KEY (session_id, seq),
    FOREIGN KEY (session_id) REFERENCES sessions(id) ON DELETE CASCADE
);
CREATE INDEX IF NOT EXISTS idx_session_messages_session ON session_messages(session_id, seq);
