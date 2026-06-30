use crate::domain::errors::AppError;
use crate::domain::permission::{PermissionPreference, PermissionResponse};
use crate::infrastructure::database::PermissionRepository;
use crate::infrastructure::permission_gate::AppPermissionGate;
use std::sync::Arc;
use tauri::State;

/// Dispatches a frontend permission response to the pending request in the gate.
///
/// Called by the frontend after the user clicks Allow/Deny/Allow Always/Deny Always
/// on a permission prompt triggered by a `"permission-required"` event.
#[tauri::command]
pub async fn respond_to_permission(
    request_id: String,
    response: PermissionResponse,
    gate: State<'_, Arc<AppPermissionGate>>,
) -> Result<(), AppError> {
    gate.resolve_permission(&request_id, response).await
}

/// Returns all persisted permission preferences.
#[tauri::command]
pub async fn get_permission_preferences(
    prefs_repo: State<'_, Arc<PermissionRepository>>,
) -> Result<Vec<PermissionPreference>, AppError> {
    prefs_repo.list_preferences().await
}

/// Persists a new permission preference. Also updates the gate's in-memory cache.
#[tauri::command]
pub async fn set_permission_preference(
    tool_name: String,
    decision: String,
    path_pattern: Option<String>,
    prefs_repo: State<'_, Arc<PermissionRepository>>,
    gate: State<'_, Arc<AppPermissionGate>>,
) -> Result<(), AppError> {
    if decision != "allow" && decision != "deny" {
        return Err(AppError::SystemError(
            "decision must be 'allow' or 'deny'".into(),
        ));
    }
    prefs_repo
        .set_preference(&tool_name, path_pattern.as_deref(), &decision)
        .await?;
    gate.reload_preferences().await?;
    Ok(())
}

/// Deletes a persisted permission preference. Also updates the gate's in-memory cache.
#[tauri::command]
pub async fn delete_permission_preference(
    tool_name: String,
    path_pattern: Option<String>,
    prefs_repo: State<'_, Arc<PermissionRepository>>,
    gate: State<'_, Arc<AppPermissionGate>>,
) -> Result<(), AppError> {
    prefs_repo
        .delete_preference(&tool_name, path_pattern.as_deref())
        .await?;
    gate.reload_preferences().await?;
    Ok(())
}
