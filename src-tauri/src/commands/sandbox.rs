use crate::domain::config::AppConfig;
use crate::domain::errors::AppError;
use crate::infrastructure::agent::try_get_shared_sandbox;
use tauri::{AppHandle, Manager, State};

#[tauri::command]
pub async fn add_sandbox_root(
    root: String,
    config: State<'_, tokio::sync::RwLock<AppConfig>>,
    app: AppHandle,
) -> Result<(), AppError> {
    {
        let config_guard = config.read().await;
        if config_guard.sandbox_roots.contains(&root) {
            return Ok(());
        }
    }

    if let Some(shared) = try_get_shared_sandbox() {
        shared
            .add_root(&root)
            .map_err(|e| AppError::SystemError(format!("Failed to add sandbox root: {}", e)))?;
    }

    {
        let mut config_guard = config.write().await;
        config_guard.sandbox_roots.push(root);
    }

    let config_guard = config.read().await;
    let config_dir = app
        .path()
        .app_config_dir()
        .map_err(|e| AppError::SystemError(format!("Failed to resolve config directory: {}", e)))?;
    let config_path = config_dir.join("config.toml");
    config_guard
        .save_to(&config_path)
        .map_err(|e| AppError::SystemError(format!("Failed to save config: {}", e)))?;

    Ok(())
}

#[tauri::command]
pub async fn remove_sandbox_root(
    root: String,
    config: State<'_, tokio::sync::RwLock<AppConfig>>,
    app: AppHandle,
) -> Result<(), AppError> {
    if let Some(shared) = try_get_shared_sandbox() {
        shared
            .remove_root(&root)
            .map_err(|e| AppError::SystemError(format!("Failed to remove sandbox root: {}", e)))?;
    }

    {
        let mut config_guard = config.write().await;
        config_guard.sandbox_roots.retain(|r| r != &root);
    }

    let config_guard = config.read().await;
    let config_dir = app
        .path()
        .app_config_dir()
        .map_err(|e| AppError::SystemError(format!("Failed to resolve config directory: {}", e)))?;
    let config_path = config_dir.join("config.toml");
    config_guard
        .save_to(&config_path)
        .map_err(|e| AppError::SystemError(format!("Failed to save config: {}", e)))?;

    Ok(())
}

#[tauri::command]
pub async fn list_sandbox_roots(
    config: State<'_, tokio::sync::RwLock<AppConfig>>,
) -> Result<Vec<String>, AppError> {
    let config_guard = config.read().await;
    let mut roots = vec![config_guard.sandbox_dir.clone()];
    roots.extend(config_guard.sandbox_roots.iter().cloned());
    Ok(roots)
}
