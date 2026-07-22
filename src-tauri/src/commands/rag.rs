use crate::domain::config::AppConfig;
use crate::domain::errors::AppError;
use crate::handlers::rag::{self as rag_handler, IndexingProgressPayload, RagTelemetryResponse};
use crate::infrastructure::rag::RagManager;
use tauri::{AppHandle, Emitter, Manager, State};

#[tauri::command]
pub async fn get_rag_telemetry(
    config: State<'_, tokio::sync::RwLock<AppConfig>>,
    manager: State<'_, RagManager>,
    app: AppHandle,
) -> Result<RagTelemetryResponse, AppError> {
    let config_guard = config.read().await;
    let app_data_dir = app
        .path()
        .app_data_dir()
        .map_err(|e| AppError::SystemError(format!("failed to resolve app data dir: {}", e)))?;
    let rag = manager.get().await;
    rag_handler::get_telemetry(&config_guard, rag.as_ref(), &app_data_dir).await
}

#[tauri::command]
pub async fn get_rag_directories(
    vault_path: String,
    config: State<'_, tokio::sync::RwLock<AppConfig>>,
) -> Result<Vec<rag_handler::DiscoveredFolderResponse>, AppError> {
    let config_guard = config.read().await;
    rag_handler::get_directories(&vault_path, &config_guard).await
}

#[tauri::command]
pub async fn toggle_rag_exclusion(
    dir_name: String,
    config: State<'_, tokio::sync::RwLock<AppConfig>>,
    manager: State<'_, RagManager>,
    app: AppHandle,
) -> Result<bool, AppError> {
    let app_data_dir = app
        .path()
        .app_data_dir()
        .map_err(|e| AppError::SystemError(format!("failed to resolve app data dir: {}", e)))?;

    let new_excluded;
    let updated_config;

    {
        let mut config_guard = config.write().await;
        let rag = manager.get_or_init(&config_guard, &app_data_dir).await?;
        new_excluded = rag_handler::toggle_exclusion(&dir_name, &mut config_guard, &rag).await?;
        updated_config = config_guard.clone();
    }

    let config_dir = app
        .path()
        .app_config_dir()
        .map_err(|e| AppError::SystemError(format!("failed to resolve config dir: {}", e)))?;
    let config_path = config_dir.join("config.toml");
    updated_config
        .save_to(&config_path)
        .map_err(|e| AppError::SystemError(format!("failed to save config: {}", e)))?;

    Ok(new_excluded)
}

#[tauri::command]
pub async fn clear_rag_database(
    config: State<'_, tokio::sync::RwLock<AppConfig>>,
    manager: State<'_, RagManager>,
    app: AppHandle,
) -> Result<(), AppError> {
    let config_guard = config.read().await;
    let app_data_dir = app
        .path()
        .app_data_dir()
        .map_err(|e| AppError::SystemError(format!("failed to resolve app data dir: {}", e)))?;
    manager.clear(&config_guard, &app_data_dir).await
}

#[tauri::command]
pub async fn query_rag_sandbox(
    query: String,
    manager: State<'_, RagManager>,
) -> Result<Vec<rag_handler::SearchResultResponse>, AppError> {
    let rag = match manager.get().await {
        Some(r) => r,
        None => return Ok(Vec::new()),
    };
    rag_handler::query_sandbox(&query, &rag).await
}

#[tauri::command]
pub async fn list_rag_directory(
    path: String,
    config: State<'_, tokio::sync::RwLock<AppConfig>>,
) -> Result<Vec<rag_handler::RagDirEntry>, AppError> {
    let config_guard = config.read().await;
    rag_handler::list_rag_directory(&path, &config_guard).await
}

#[tauri::command]
pub async fn read_rag_document(
    path: String,
    config: State<'_, tokio::sync::RwLock<AppConfig>>,
) -> Result<String, AppError> {
    let config_guard = config.read().await;
    rag_handler::read_rag_document(&path, &config_guard).await
}

#[tauri::command]
pub async fn remove_rag_dir(
    dir_path: String,
    config: State<'_, tokio::sync::RwLock<AppConfig>>,
    manager: State<'_, RagManager>,
    app: AppHandle,
) -> Result<(), AppError> {
    let updated_config;

    {
        let mut config_guard = config.write().await;
        let rag = manager.get().await;
        rag_handler::remove_rag_dir(&dir_path, &mut config_guard, rag.as_deref()).await?;
        updated_config = config_guard.clone();
    }

    let config_dir = app
        .path()
        .app_config_dir()
        .map_err(|e| AppError::SystemError(format!("failed to resolve config dir: {}", e)))?;
    let config_path = config_dir.join("config.toml");
    updated_config
        .save_to(&config_path)
        .map_err(|e| AppError::SystemError(format!("failed to save config: {}", e)))?;

    Ok(())
}

#[tauri::command]
pub async fn start_rag_indexing(
    vault_path: String,
    config: State<'_, tokio::sync::RwLock<AppConfig>>,
    manager: State<'_, RagManager>,
    app: AppHandle,
) -> Result<(), AppError> {
    let app_data_dir = app
        .path()
        .app_data_dir()
        .map_err(|e| AppError::SystemError(format!("failed to resolve app data dir: {}", e)))?;

    let config_snapshot = config.read().await.clone();
    let rag = manager.get_or_init(&config_snapshot, &app_data_dir).await?;

    let emit = |payload: IndexingProgressPayload| {
        let _ = app.emit("rag-status-update", payload);
    };

    rag_handler::start_indexing(&vault_path, &config_snapshot, &rag, &app_data_dir, emit).await
}
