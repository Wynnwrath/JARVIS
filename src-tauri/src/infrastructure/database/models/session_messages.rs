use crate::infrastructure::database::models::schema::session_messages;
use diesel::prelude::*;
use serde::{Deserialize, Serialize};

/// Message role stored in the `session_messages` table.
///
/// Maps 1-to-1 with rig_core's message roles as serialized strings.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "PascalCase")]
pub enum MessageRole {
    User,
    Assistant,
    Tool,
    System,
}

impl MessageRole {
    pub fn as_str(&self) -> &'static str {
        match self {
            MessageRole::User => "User",
            MessageRole::Assistant => "Assistant",
            MessageRole::Tool => "Tool",
            MessageRole::System => "System",
        }
    }
}

impl std::fmt::Display for MessageRole {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "{}", self.as_str())
    }
}

impl std::str::FromStr for MessageRole {
    type Err = String;

    fn from_str(s: &str) -> Result<Self, Self::Err> {
        match s {
            "User" => Ok(MessageRole::User),
            "Assistant" => Ok(MessageRole::Assistant),
            "Tool" => Ok(MessageRole::Tool),
            "System" => Ok(MessageRole::System),
            _ => Err(format!("Unknown message role: {}", s)),
        }
    }
}

/// A single normalized message row in `session_messages`.
#[derive(Queryable, Identifiable, Selectable, PartialEq, Debug, Clone)]
#[diesel(table_name = session_messages)]
#[diesel(primary_key(session_id, seq))]
#[diesel(check_for_backend(diesel::sqlite::Sqlite))]
pub struct SessionMessageRow {
    pub session_id: String,
    pub seq: i32,
    pub role: String,
    pub content_json: String,
    pub created_at: i64,
}

/// Insertable form of [`SessionMessageRow`].
#[derive(Insertable, PartialEq, Debug, Clone)]
#[diesel(table_name = session_messages)]
pub struct NewSessionMessageRow<'a> {
    pub session_id: &'a str,
    pub seq: i32,
    pub role: &'a str,
    pub content_json: &'a str,
    pub created_at: i64,
}
