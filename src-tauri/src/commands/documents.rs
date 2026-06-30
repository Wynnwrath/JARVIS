use crate::domain::config::AppConfig;
use crate::domain::errors::AppError;
use crate::handlers::documents;
use crate::infrastructure::permission_gate::AppPermissionGate;
use std::sync::Arc;
use tauri::{Manager, State};

/// Reads a file inside the sandbox directory, enforcing the configured read extensions.
///
/// # Arguments
///
/// * `path` - Relative path to the file within the sandbox.
/// * `config` - The application configuration state, injected by Tauri.
///
/// # Returns
///
/// Returns the file contents as a string on success, or an [`AppError`] on failure.
///
/// # Errors
///
/// Returns [`AppError::SystemError`] if the file cannot be read or its extension is
/// not in the allowed read extensions list.
#[tauri::command]
pub async fn read_document(
    path: String,
    config: State<'_, tokio::sync::RwLock<AppConfig>>,
    app: tauri::AppHandle,
) -> Result<String, AppError> {
    let read_extensions = {
        let guard = config.read().await;
        guard.read_extensions.clone()
    };
    let gate: Option<Arc<AppPermissionGate>> = app
        .try_state::<Arc<AppPermissionGate>>()
        .map(|s| s.inner().clone());
    documents::read_document(read_extensions, path, gate).await
}

/// Writes content to a file inside the sandbox, enforcing write extensions.
///
/// When `append` is `true`, the content is appended to the existing file rather
/// than overwriting it.
///
/// # Arguments
///
/// * `path` - Relative path to the file within the sandbox.
/// * `content` - The text content to write.
/// * `append` - If `true`, append instead of overwrite. Defaults to `false` when `None`.
/// * `config` - The application configuration state, injected by Tauri.
///
/// # Returns
///
/// Returns a confirmation message string on success, or an [`AppError`] on failure.
///
/// # Errors
///
/// Returns [`AppError::SystemError`] if the file cannot be written or its extension is
/// not in the allowed write extensions list.
#[tauri::command]
pub async fn write_document(
    path: String,
    content: String,
    append: Option<bool>,
    config: State<'_, tokio::sync::RwLock<AppConfig>>,
    app: tauri::AppHandle,
) -> Result<String, AppError> {
    let write_extensions = {
        let guard = config.read().await;
        guard.write_extensions.clone()
    };
    let gate: Option<Arc<AppPermissionGate>> = app
        .try_state::<Arc<AppPermissionGate>>()
        .map(|s| s.inner().clone());
    documents::write_document(write_extensions, path, content, append, gate).await
}

/// Lists entries at a path inside the sandbox directory.
///
/// If `path` is `None`, lists the sandbox root.
///
/// # Arguments
///
/// * `path` - Optional relative path to list within the sandbox.
///
/// # Returns
///
/// Returns a formatted directory listing string on success, or an [`AppError`] on failure.
///
/// # Errors
///
/// Returns [`AppError::SystemError`] if the directory cannot be read.
#[tauri::command]
pub async fn list_directory(
    path: Option<String>,
    app: tauri::AppHandle,
) -> Result<String, AppError> {
    let gate: Option<Arc<AppPermissionGate>> = app
        .try_state::<Arc<AppPermissionGate>>()
        .map(|s| s.inner().clone());
    documents::list_directory(path, gate).await
}

/// Glob-searches for files matching `pattern` inside the sandbox.
///
/// # Arguments
///
/// * `pattern` - A glob pattern (e.g. `"**/*.rs"`, `"*.toml"`).
///
/// # Returns
///
/// Returns a JSON array of matching file paths as a string on success,
/// or an [`AppError`] on failure.
///
/// # Errors
///
/// Returns [`AppError::SystemError`] if the glob pattern is invalid or the search fails.
#[tauri::command]
pub async fn glob_search(pattern: String, app: tauri::AppHandle) -> Result<String, AppError> {
    let gate: Option<Arc<AppPermissionGate>> = app
        .try_state::<Arc<AppPermissionGate>>()
        .map(|s| s.inner().clone());
    documents::glob_search(pattern, gate).await
}

/// Grep-searches file contents inside the sandbox for a given query.
///
/// `path` restricts the search to a subdirectory; `case_sensitive` defaults to `false`.
///
/// # Arguments
///
/// * `query` - The text or regex pattern to search for in file contents.
/// * `path` - Optional subdirectory to restrict the search within the sandbox.
/// * `case_sensitive` - Whether the search should be case-sensitive. Defaults to `false`.
/// * `config` - The application configuration state, injected by Tauri.
///
/// # Returns
///
/// Returns a JSON-formatted string of matching file paths with line numbers on success,
/// or an [`AppError`] on failure.
///
/// # Errors
///
/// Returns [`AppError::SystemError`] if the search fails or the sandbox configuration
/// is invalid.
#[tauri::command]
pub async fn grep_search(
    query: String,
    path: Option<String>,
    case_sensitive: Option<bool>,
    config: State<'_, tokio::sync::RwLock<AppConfig>>,
    app: tauri::AppHandle,
) -> Result<String, AppError> {
    let read_extensions = {
        let guard = config.read().await;
        guard.read_extensions.clone()
    };
    let gate: Option<Arc<AppPermissionGate>> = app
        .try_state::<Arc<AppPermissionGate>>()
        .map(|s| s.inner().clone());
    documents::grep_search(read_extensions, query, path, case_sensitive, gate).await
}
