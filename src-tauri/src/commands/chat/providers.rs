use crate::domain::config::AppConfig;
use crate::domain::errors::AppError;
use crate::handlers::chat::{get_providers, set_provider};
use tauri::State;

/// Retrieves the list of LLM provider names supported by the application.
///
/// These values are safe to pass directly to [`set_chat_provider`].
///
/// # Returns
///
/// Returns a vector of provider name strings (`"openai"`, `"gemini"`, `"anthropic"`)
/// on success, or an [`AppError`] on failure.
#[tauri::command]
pub async fn get_chat_providers() -> Result<Vec<String>, AppError> {
    get_providers()
}

/// Switches the active LLM provider and persists the change to the config file.
///
/// Accepts one of `"openai"`, `"gemini"`, or `"anthropic"`. The config mutex is
/// released before the file write so concurrent reads are not blocked during I/O.
///
/// # Arguments
///
/// * `provider` - The provider name to activate. Must match one of the names
///   returned by [`get_chat_providers`].
/// * `config` - The application configuration state.
/// * `app` - The Tauri application handle, used to resolve the config directory path.
///
/// # Returns
///
/// Returns `Ok(())` on success, or an [`AppError`] on failure.
///
/// # Errors
///
/// Returns [`AppError::SystemError`] if the provider name is unknown or the config
/// file cannot be written.
#[tauri::command]
pub async fn set_chat_provider(
    provider: String,
    config: State<'_, tokio::sync::RwLock<AppConfig>>,
    app: tauri::AppHandle,
) -> Result<(), AppError> {
    use tauri::Manager;
    let config_path = app
        .path()
        .app_config_dir()
        .ok()
        .map(|dir| dir.join("config.toml"));
    set_provider(provider, &config, config_path.as_deref()).await
}
