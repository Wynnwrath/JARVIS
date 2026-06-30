use crate::domain::chat::Session;
use crate::domain::errors::AppError;
use crate::infrastructure::database::{
    global_pool,
    models::{
        schema::{
            session_history::dsl as hist, session_messages::dsl as msgs, sessions::dsl as sess,
        },
        MessageRole, NewSessionHistoryRow, NewSessionMessageRow, NewSessionRow, SessionHistoryRow,
        SessionMessageRow, SessionRow,
    },
    DbPool,
};
use diesel::prelude::*;
use diesel_async::{AsyncConnection, RunQueryDsl};
use rig_core::message::Message;
use uuid::Uuid;

pub struct SessionRepository {
    pool: DbPool,
}

impl Default for SessionRepository {
    fn default() -> Self {
        Self::new()
    }
}

impl SessionRepository {
    pub fn new() -> Self {
        Self {
            pool: global_pool(),
        }
    }

    pub fn with_pool(pool: DbPool) -> Self {
        Self { pool }
    }

    pub async fn create_session(&self, title: Option<String>) -> Result<String, AppError> {
        let id = Uuid::new_v4().to_string();
        let now = chrono::Utc::now().timestamp();
        let mut conn = self.pool.get().await?;
        conn.transaction::<_, AppError, _>(async |conn| {
            diesel::insert_into(sess::sessions)
                .values(NewSessionRow {
                    id: id.clone(),
                    title,
                    created_at: now,
                    updated_at: now,
                })
                .execute(conn)
                .await?;
            diesel::insert_into(hist::session_history)
                .values(NewSessionHistoryRow {
                    session_id: &id,
                    history_json: "[]",
                })
                .execute(conn)
                .await?;
            Ok(())
        })
        .await?;
        Ok(id)
    }

    /// Read session history from the normalized `session_messages` table.
    ///
    /// Falls back to the legacy `history_json` column for sessions last written
    /// by the old schema: when `session_messages` is empty but `history_json`
    /// contains data, the JSON is parsed, inserted into `session_messages`, and
    /// the legacy column is left untouched (read-only fallback).
    pub async fn get_session_history(&self, session_id: &str) -> Result<Vec<Message>, AppError> {
        let mut conn = self.pool.get().await?;

        // 1. Try the normalized table first.
        let rows: Vec<SessionMessageRow> = msgs::session_messages
            .filter(msgs::session_id.eq(session_id))
            .order(msgs::seq.asc())
            .load::<SessionMessageRow>(&mut conn)
            .await?;

        if !rows.is_empty() {
            return rows
                .into_iter()
                .map(|r| {
                    let role = r
                        .role
                        .parse::<MessageRole>()
                        .map_err(AppError::SystemError)?;
                    let msg: Message = serde_json::from_str(&r.content_json)?;
                    Ok((role, msg))
                })
                .map(|r| r.map(|(_, msg)| msg))
                .collect::<Result<Vec<_>, _>>();
        }

        // 2. Fallback: read from legacy `history_json` and backfill.
        let row: Option<SessionHistoryRow> = hist::session_history
            .filter(hist::session_id.eq(session_id))
            .first::<SessionHistoryRow>(&mut conn)
            .await
            .optional()?;

        let row = row.ok_or_else(|| AppError::SystemError("Session not found".into()))?;

        let history: Vec<Message> = serde_json::from_str(&row.history_json)?;

        if !history.is_empty() {
            let now = chrono::Utc::now().timestamp();
            for (i, msg) in history.iter().enumerate() {
                let role = message_role(msg);
                let content_json = serde_json::to_string(msg).unwrap_or_else(|_| "{}".to_string());
                diesel::insert_into(msgs::session_messages)
                    .values(NewSessionMessageRow {
                        session_id,
                        seq: i as i32,
                        role: role.as_str(),
                        content_json: &content_json,
                        created_at: now,
                    })
                    .execute(&mut conn)
                    .await?;
            }
        }

        Ok(history)
    }

    /// Persist messages by inserting only the new ones (upsert by session_id + seq).
    ///
    /// The `sessions.updated_at` timestamp is bumped in the same transaction.
    pub async fn save_session_history(
        &self,
        session_id: &str,
        history: &[Message],
    ) -> Result<(), AppError> {
        let now = chrono::Utc::now().timestamp();
        let mut conn = self.pool.get().await?;
        conn.transaction::<_, AppError, _>(async |conn| {
            // Determine the next sequence number.
            let max_seq: Option<i32> = msgs::session_messages
                .filter(msgs::session_id.eq(session_id))
                .select(diesel::dsl::max(msgs::seq))
                .first::<Option<i32>>(conn)
                .await
                .optional()?
                .flatten();

            let start = max_seq.map(|s| s + 1).unwrap_or(0);

            // Insert only messages beyond the current max seq.
            for (i, msg) in history.iter().enumerate().skip(start as usize) {
                let role = message_role(msg);
                let content_json = serde_json::to_string(msg).unwrap_or_else(|_| "{}".to_string());
                diesel::insert_into(msgs::session_messages)
                    .values(NewSessionMessageRow {
                        session_id,
                        seq: i as i32,
                        role: role.as_str(),
                        content_json: &content_json,
                        created_at: now,
                    })
                    .execute(conn)
                    .await?;
            }

            let n = diesel::update(sess::sessions.filter(sess::id.eq(session_id)))
                .set(sess::updated_at.eq(now))
                .execute(conn)
                .await?;
            if n == 0 {
                return Err(AppError::SystemError("Session not found".into()));
            }
            Ok(())
        })
        .await
    }

    /// Compact a session's message history by replacing older messages with a
    /// summary row.
    ///
    /// Removes all rows with `seq <= up_to_seq` and inserts a single summary
    /// message at `seq = 0`. The remaining rows are renumbered starting from 1.
    pub async fn compact_session_history(
        &self,
        session_id: &str,
        summary_msg: &Message,
        up_to_seq: i32,
    ) -> Result<(), AppError> {
        let now = chrono::Utc::now().timestamp();
        let mut conn = self.pool.get().await?;
        conn.transaction::<_, AppError, _>(async |conn| {
            // 1. Delete all rows up to and including up_to_seq.
            diesel::delete(
                msgs::session_messages
                    .filter(msgs::session_id.eq(session_id))
                    .filter(msgs::seq.le(up_to_seq)),
            )
            .execute(conn)
            .await?;

            // 2. Shift remaining rows down so seq restarts at 1 (not 0, which is
            //    reserved for the summary). Use raw SQL since Diesel's update
            //    doesn't support self-referential arithmetic on SQLite without subqueries.
            //    After deleting rows <= up_to_seq, remaining rows start at up_to_seq+1.
            //    Subtracting up_to_seq maps them to 1, 2, 3, ... leaving seq 0 free.
            diesel::sql_query(format!(
                "UPDATE session_messages SET seq = seq - {} WHERE session_id = ?1",
                up_to_seq
            ))
            .bind::<diesel::sql_types::Text, _>(session_id)
            .execute(conn)
            .await?;

            // 3. Insert the summary at seq 0 (no collision since seq 0 is free).
            let role = message_role(summary_msg);
            let content_json =
                serde_json::to_string(summary_msg).unwrap_or_else(|_| "{}".to_string());

            diesel::insert_into(msgs::session_messages)
                .values(NewSessionMessageRow {
                    session_id,
                    seq: 0,
                    role: role.as_str(),
                    content_json: &content_json,
                    created_at: now,
                })
                .execute(conn)
                .await?;

            Ok(())
        })
        .await
    }

    pub async fn get_all_sessions(&self) -> Result<Vec<Session>, AppError> {
        let mut conn = self.pool.get().await?;
        let rows: Vec<SessionRow> = sess::sessions
            .order(sess::updated_at.desc())
            .load::<SessionRow>(&mut conn)
            .await?;
        Ok(rows.into_iter().map(Session::from).collect())
    }

    pub async fn rename_session(&self, session_id: &str, title: &str) -> Result<(), AppError> {
        let now = chrono::Utc::now().timestamp();
        let mut conn = self.pool.get().await?;
        let n = diesel::update(sess::sessions.filter(sess::id.eq(session_id)))
            .set((sess::title.eq(title), sess::updated_at.eq(now)))
            .execute(&mut conn)
            .await?;
        if n == 0 {
            return Err(AppError::SystemError("Session not found".into()));
        }
        Ok(())
    }

    pub async fn delete_session(&self, session_id: &str) -> Result<(), AppError> {
        let mut conn = self.pool.get().await?;
        let n = diesel::delete(sess::sessions.filter(sess::id.eq(session_id)))
            .execute(&mut conn)
            .await?;
        if n == 0 {
            return Err(AppError::SystemError("Session not found".into()));
        }
        Ok(())
    }
}

/// Map a `rig_core::message::Message` variant to the normalized role string.
fn message_role(msg: &Message) -> MessageRole {
    match msg {
        Message::User { .. } => MessageRole::User,
        Message::Assistant { .. } => MessageRole::Assistant,
        Message::System { .. } => MessageRole::System,
    }
}
