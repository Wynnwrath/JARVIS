use crate::domain::chat::Session;
use crate::domain::errors::AppError;
use crate::infrastructure::database::SessionRepository;

/// Creates a new chat session with an optional human-readable title.
///
/// Delegates to [`SessionRepository::create_session`], which inserts the session
/// and an empty history row inside a single SQLite transaction.
///
/// # Arguments
///
/// * `title` - An optional display name for the new session. Pass `None` for an untitled session.
///
/// # Returns
///
/// Returns the unique session ID (UUID v4) on success, or an [`AppError`] on failure.
///
/// # Errors
///
/// Returns [`AppError::LockError`] if the database mutex is poisoned.
/// Returns [`AppError::SystemError`] if the SQL INSERT or transaction commit fails.
#[tauri::command]
pub async fn create_session(title: Option<String>) -> Result<String, AppError> {
    SessionRepository::new().create_session(title).await
}

/// Lists all chat sessions ordered by most-recently updated first.
///
/// Delegates to [`SessionRepository::get_all_sessions`].
///
/// # Arguments
///
/// # Returns
///
/// Returns a vector of [`Session`] structs on success, or an [`AppError`] on failure.
///
/// # Errors
///
/// Returns [`AppError::LockError`] if the database mutex is poisoned.
/// Returns [`AppError::SystemError`] on SQL query failures.
#[tauri::command]
pub async fn list_sessions() -> Result<Vec<Session>, AppError> {
    SessionRepository::new().get_all_sessions().await
}

/// Retrieves the message history for a specific chat session.
///
/// Delegates to [`SessionRepository::get_session_history`].
///
/// # Arguments
///
/// * `session_id` - The unique identifier of the session to retrieve history for.
///
/// # Returns
///
/// Returns a vector of [`Message`]s representing the conversation history on success,
/// or an [`AppError`] on failure.
///
/// # Errors
///
/// Returns [`AppError::SystemError("Session not found")`] if `session_id` does not exist.
/// Returns [`AppError::LockError`] on mutex poison.
/// Returns [`AppError::SystemError`] on JSON deserialisation failures.
#[tauri::command]
pub async fn get_history(session_id: String) -> Result<Vec<rig_core::message::Message>, AppError> {
    SessionRepository::new()
        .get_session_history(&session_id)
        .await
}

/// Renames a chat session and bumps its `updated_at` timestamp.
///
/// Delegates to [`SessionRepository::rename_session`].
///
/// # Arguments
///
/// * `session_id` - The unique identifier of the session to rename.
/// * `title` - The new display title for the session.
///
/// # Returns
///
/// Returns `Ok(())` on success, or an [`AppError`] on failure.
///
/// # Errors
///
/// Returns [`AppError::SystemError("Session not found")`] if `session_id` does not exist.
/// Returns [`AppError::LockError`] on mutex poison.
#[tauri::command]
pub async fn rename_session(session_id: String, title: String) -> Result<(), AppError> {
    SessionRepository::new()
        .rename_session(&session_id, &title)
        .await
}

/// Deletes a chat session and all its associated history (cascaded via foreign key).
///
/// Delegates to [`SessionRepository::delete_session`].
///
/// # Arguments
///
/// * `session_id` - The unique identifier of the session to delete.
///
/// # Returns
///
/// Returns `Ok(())` on success, or an [`AppError`] on failure.
///
/// # Errors
///
/// Returns [`AppError::SystemError("Session not found")`] if `session_id` does not exist.
/// Returns [`AppError::LockError`] on mutex poison.
#[tauri::command]
pub async fn delete_session(session_id: String) -> Result<(), AppError> {
    SessionRepository::new().delete_session(&session_id).await
}
