//! Regression tests for Phase 8: session_messages normalization.
//!
//! Verifies backfill from legacy `history_json`, correct seq ordering after
//! `save_session_history`, and `compact_session_history` collision fix.

use jarvis_lib::infrastructure::database::{create_pool, run_migrations, SessionRepository};
use rig_core::message::Message;
use std::fs;

fn cleanup(path: &std::path::Path) {
    let _ = fs::remove_file(path);
    let _ = fs::remove_file(format!("{}-wal", path.display()));
    let _ = fs::remove_file(format!("{}-shm", path.display()));
}

fn make_old_schema_db(path: &std::path::Path) {
    let conn = rusqlite::Connection::open(path).unwrap();
    conn.execute_batch("PRAGMA foreign_keys = ON;").unwrap();
    conn.execute(
        "CREATE TABLE IF NOT EXISTS sessions (
            id TEXT PRIMARY KEY,
            title TEXT,
            created_at INTEGER NOT NULL,
            updated_at INTEGER NOT NULL
        )",
        [],
    )
    .unwrap();
    conn.execute(
        "CREATE TABLE IF NOT EXISTS session_history (
            session_id TEXT PRIMARY KEY,
            history_json TEXT NOT NULL,
            FOREIGN KEY(session_id) REFERENCES sessions(id) ON DELETE CASCADE
        )",
        [],
    )
    .unwrap();
    // Create the diesel migrations table and mark 0001 as already applied so
    // run_migrations only runs 0002_session_messages_normalized.
    conn.execute_batch(
        "CREATE TABLE IF NOT EXISTS __diesel_schema_migrations (
            version VARCHAR(50) PRIMARY KEY NOT NULL,
            run_on TIMESTAMP NOT NULL DEFAULT CURRENT_TIMESTAMP
        );
        INSERT INTO __diesel_schema_migrations (version) VALUES ('0001');",
    )
    .unwrap();
    drop(conn);
}

fn extract_text(msg: &Message) -> String {
    match msg {
        Message::System { content, .. } => content.clone(),
        Message::User { content, .. } => content
            .iter()
            .filter_map(|c| match c {
                rig_core::message::UserContent::Text(t) => Some(t.text.as_str()),
                _ => None,
            })
            .collect::<Vec<_>>()
            .join(""),
        Message::Assistant { content, .. } => content
            .iter()
            .filter_map(|c| match c {
                rig_core::message::AssistantContent::Text(t) => Some(t.text.as_str()),
                _ => None,
            })
            .collect::<Vec<_>>()
            .join(""),
    }
}

fn assert_messages_match_text(actual: &[Message], expected_texts: &[&str]) {
    assert_eq!(actual.len(), expected_texts.len(), "message count mismatch");
    for (i, (msg, expected)) in actual.iter().zip(expected_texts.iter()).enumerate() {
        assert_eq!(extract_text(msg), *expected, "text mismatch at index {}", i);
    }
}

#[tokio::test]
async fn backfill_from_history_json_preserves_messages() {
    let path = std::env::temp_dir().join("jarvis_norm_backfill.db");
    cleanup(&path);

    make_old_schema_db(&path);
    let messages = vec![
        Message::user("Hello"),
        Message::assistant("Hi there!"),
        Message::user("How are you?"),
        Message::assistant("I'm doing well."),
    ];
    let history_json = serde_json::to_string(&messages).unwrap();

    let conn = rusqlite::Connection::open(&path).unwrap();
    conn.execute(
        "INSERT INTO sessions (id, title, created_at, updated_at) VALUES ('s1', 'Test', 100, 100)",
        [],
    )
    .unwrap();
    conn.execute(
        "INSERT INTO session_history (session_id, history_json) VALUES ('s1', ?1)",
        [&history_json],
    )
    .unwrap();
    drop(conn);

    run_migrations(path.to_str().unwrap()).expect("run_migrations failed");

    let pool = create_pool(path.to_str().unwrap());
    let repo = SessionRepository::with_pool(pool);
    let loaded = repo.get_session_history("s1").await.unwrap();

    assert_messages_match_text(
        &loaded,
        &["Hello", "Hi there!", "How are you?", "I'm doing well."],
    );

    // Verify session_messages rows were backfilled with correct seq and role.
    let conn = rusqlite::Connection::open(path.to_str().unwrap()).unwrap();
    let rows: Vec<(i32, String)> = conn
        .prepare("SELECT seq, role FROM session_messages WHERE session_id = 's1' ORDER BY seq")
        .unwrap()
        .query_map([], |r| Ok((r.get(0)?, r.get(1)?)))
        .unwrap()
        .filter_map(|r| r.ok())
        .collect();
    assert_eq!(rows.len(), 4);
    assert_eq!(rows[0], (0, "User".to_string()));
    assert_eq!(rows[1], (1, "Assistant".to_string()));
    assert_eq!(rows[2], (2, "User".to_string()));
    assert_eq!(rows[3], (3, "Assistant".to_string()));
    drop(conn);

    // Re-calling get_session_history reads from session_messages (no re-backfill).
    let loaded2 = repo.get_session_history("s1").await.unwrap();
    assert_messages_match_text(
        &loaded2,
        &["Hello", "Hi there!", "How are you?", "I'm doing well."],
    );

    cleanup(&path);
}

#[tokio::test]
async fn save_session_history_appends_with_correct_seq() {
    let path = std::env::temp_dir().join("jarvis_norm_save_append.db");
    cleanup(&path);

    make_old_schema_db(&path);
    let initial = vec![
        Message::user("msg0"),
        Message::assistant("msg1"),
        Message::user("msg2"),
    ];
    let history_json = serde_json::to_string(&initial).unwrap();

    let conn = rusqlite::Connection::open(&path).unwrap();
    conn.execute(
        "INSERT INTO sessions (id, title, created_at, updated_at) VALUES ('s2', 'T', 100, 100)",
        [],
    )
    .unwrap();
    conn.execute(
        "INSERT INTO session_history (session_id, history_json) VALUES ('s2', ?1)",
        [&history_json],
    )
    .unwrap();
    drop(conn);

    run_migrations(path.to_str().unwrap()).expect("run_migrations failed");

    let pool = create_pool(path.to_str().unwrap());
    let repo = SessionRepository::with_pool(pool);

    // Trigger backfill.
    let loaded = repo.get_session_history("s2").await.unwrap();
    assert_eq!(loaded.len(), 3);

    // Append one new message and save.
    let mut full_history: Vec<Message> = loaded.clone();
    full_history.push(Message::user("msg3_new"));
    repo.save_session_history("s2", &full_history)
        .await
        .unwrap();

    // Verify exactly 4 rows (3 original + 1 new), no double-counting.
    let conn = rusqlite::Connection::open(path.to_str().unwrap()).unwrap();
    let count: i32 = conn
        .query_row(
            "SELECT COUNT(*) FROM session_messages WHERE session_id = 's2'",
            [],
            |r| r.get(0),
        )
        .unwrap();
    assert_eq!(
        count, 4,
        "should have exactly 4 rows after appending one message"
    );

    let seqs: Vec<i32> = conn
        .prepare("SELECT seq FROM session_messages WHERE session_id = 's2' ORDER BY seq")
        .unwrap()
        .query_map([], |r| r.get(0))
        .unwrap()
        .filter_map(|r| r.ok())
        .collect();
    assert_eq!(
        seqs,
        vec![0, 1, 2, 3],
        "seqs must be contiguous starting at 0"
    );
    drop(conn);

    // Re-load and verify content.
    let loaded = repo.get_session_history("s2").await.unwrap();
    assert_messages_match_text(&loaded, &["msg0", "msg1", "msg2", "msg3_new"]);

    cleanup(&path);
}

#[tokio::test]
async fn compact_session_history_no_collision() {
    let path = std::env::temp_dir().join("jarvis_norm_compact.db");
    cleanup(&path);

    make_old_schema_db(&path);
    let initial: Vec<Message> = (0..10)
        .map(|i| Message::user(format!("msg{}", i)))
        .collect();
    let history_json = serde_json::to_string(&initial).unwrap();

    let conn = rusqlite::Connection::open(path.to_str().unwrap()).unwrap();
    conn.execute(
        "INSERT INTO sessions (id, title, created_at, updated_at) VALUES ('s3', 'T', 100, 100)",
        [],
    )
    .unwrap();
    conn.execute(
        "INSERT INTO session_history (session_id, history_json) VALUES ('s3', ?1)",
        [&history_json],
    )
    .unwrap();
    drop(conn);

    run_migrations(path.to_str().unwrap()).expect("run_migrations failed");

    let pool = create_pool(path.to_str().unwrap());
    let repo = SessionRepository::with_pool(pool);

    // Trigger backfill.
    let loaded = repo.get_session_history("s3").await.unwrap();
    assert_eq!(loaded.len(), 10);

    // Compact: replace seq 0..=6 with a summary, keep seq 7,8,9.
    let summary = Message::assistant("Summary of first 7 messages");
    repo.compact_session_history("s3", &summary, 6)
        .await
        .unwrap();

    // After compact: seq 0 = summary, seq 1,2,3 = former 7,8,9.
    let conn = rusqlite::Connection::open(path.to_str().unwrap()).unwrap();
    let rows: Vec<(i32, String, String)> = conn
        .prepare("SELECT seq, role, content_json FROM session_messages WHERE session_id = 's3' ORDER BY seq")
        .unwrap()
        .query_map([], |r| Ok((r.get(0)?, r.get(1)?, r.get(2)?)))
        .unwrap()
        .filter_map(|r| r.ok())
        .collect();
    assert_eq!(rows.len(), 4, "should have summary + 3 remaining messages");
    assert_eq!(rows[0].0, 0, "summary at seq 0");
    assert_eq!(rows[0].1, "Assistant");
    assert_eq!(rows[1].0, 1, "first remaining at seq 1");
    assert_eq!(rows[2].0, 2);
    assert_eq!(rows[3].0, 3);

    // Verify remaining messages are the original 7, 8, 9.
    let msg7: Message = serde_json::from_str(&rows[1].2).unwrap();
    let msg8: Message = serde_json::from_str(&rows[2].2).unwrap();
    let msg9: Message = serde_json::from_str(&rows[3].2).unwrap();
    assert_eq!(extract_text(&msg7), "msg7");
    assert_eq!(extract_text(&msg8), "msg8");
    assert_eq!(extract_text(&msg9), "msg9");
    drop(conn);

    // Verify history_json is in sync.
    let loaded = repo.get_session_history("s3").await.unwrap();
    assert_eq!(loaded.len(), 4);
    assert_eq!(extract_text(&loaded[0]), "Summary of first 7 messages");
    assert_messages_match_text(&loaded[1..], &["msg7", "msg8", "msg9"]);

    cleanup(&path);
}
