use crate::domain::config::{AppConfig, Providers};
use crate::domain::errors::AppError;
use std::path::Path;

/// Returns the list of all supported LLM provider names as strings.
///
/// These values are safe to pass to [`set_provider`] and to the Tauri
/// [`set_chat_provider`](crate::commands::chat::set_chat_provider) command.
pub fn get_providers() -> Result<Vec<String>, AppError> {
    Ok(Providers::all()
        .into_iter()
        .map(|p| p.to_string())
        .collect())
}

/// Switches the active provider in the config mutex and persists to disk.
///
/// The lock is released before the file write to avoid blocking concurrent readers.
///
/// # Arguments
///
/// * `provider` - The provider name to activate (`"openai"`, `"gemini"`, or `"anthropic"`).
/// * `config` - The `tokio::sync::Mutex`-guarded application configuration.
/// * `config_path` - Optional filesystem path to persist the updated config to.
///   When `None`, the config is updated in memory only.
///
/// # Returns
///
/// Returns `Ok(())` on success, or an [`AppError`] on failure.
///
/// # Errors
///
/// Returns [`AppError::SystemError`] if the provider name is unknown or the config
/// file cannot be written.
pub async fn set_provider(
    provider: String,
    config: &tokio::sync::RwLock<AppConfig>,
    config_path: Option<&Path>,
) -> Result<(), AppError> {
    let provider_enum = provider
        .parse::<Providers>()
        .map_err(AppError::SystemError)?;

    let config_to_save = {
        let mut config_guard = config.write().await;
        config_guard.provider = provider_enum;
        config_guard.clone()
    };

    if let Some(path) = config_path {
        config_to_save
            .save_to(path)
            .map_err(|e| AppError::SystemError(format!("Failed to save config: {}", e)))?;
    }

    Ok(())
}
