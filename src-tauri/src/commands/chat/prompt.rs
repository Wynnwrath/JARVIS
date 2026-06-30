use crate::domain::chat::{ChatResponse, StreamEvent};
use crate::domain::config::AppConfig;
use crate::domain::errors::AppError;
use crate::handlers::chat::{send_prompt, send_stream_prompt};
use crate::infrastructure::database::SessionRepository;
use tauri::State;

/// Sends a user prompt to the LLM agent and returns the reply along with the active provider name.
///
/// 1. Creates a [`SessionRepository`] and loads the conversation history for `session_id`.
/// 2. Clones the current config outside the lock so the mutex is held only briefly.
/// 3. Forwards the `tauri::AppHandle` to the handler so that MCP connection errors during
///    agent (re)building can be emitted as `"mcp-connection-error"` Tauri events.
/// 4. Persists the updated history (with attachment metadata cleaned from the user message)
///    back to the database.
///
/// # Arguments
///
/// * `session_id` - The unique identifier of the session to continue.
/// * `input` - The user's prompt text.
/// * `attachments` - Optional file paths to attach as document hints for the agent.
/// * `config` - The application configuration state (locked briefly to clone).
/// * `app` - The Tauri application handle, used to emit MCP error events.
///
/// # Returns
///
/// Returns a [`ChatResponse`] containing the agent's reply and the active provider name,
/// or an [`AppError`] on failure.
///
/// # Errors
///
/// Propagates errors from history retrieval, agent chat, or history persistence.
/// Returns [`AppError::SystemError`] if the agent cannot be built or the LLM call fails.
#[tauri::command]
pub async fn prompt(
    session_id: String,
    input: String,
    attachments: Option<Vec<String>>,
    config: State<'_, tokio::sync::RwLock<AppConfig>>,
    app: tauri::AppHandle,
) -> Result<ChatResponse, AppError> {
    let config_clone = {
        let config_guard = config.read().await;
        config_guard.clone()
    };
    let provider = config_clone.provider.to_string();
    let repo = SessionRepository::new();
    let response = send_prompt(
        &session_id,
        &input,
        attachments.as_deref(),
        &config_clone,
        &repo,
        Some(&app),
    )
    .await?;

    Ok(ChatResponse {
        message: response,
        provider,
    })
}

/// Sends a user prompt to the LLM agent, streaming token chunks through the provided channel.
///
/// Returns the completed assistant reply on success, or an [`AppError`] on failure.
#[tauri::command]
pub async fn stream_prompt(
    session_id: String,
    input: String,
    attachments: Option<Vec<String>>,
    config: State<'_, tokio::sync::RwLock<AppConfig>>,
    app: tauri::AppHandle,
    channel: tauri::ipc::Channel<StreamEvent>,
) -> Result<ChatResponse, AppError> {
    let config_clone = {
        let config_guard = config.read().await;
        config_guard.clone()
    };
    let provider = config_clone.provider.to_string();
    let repo = SessionRepository::new();
    let response = send_stream_prompt(
        &session_id,
        &input,
        attachments.as_deref(),
        &config_clone,
        &repo,
        Some(&app),
        channel,
    )
    .await?;

    Ok(ChatResponse {
        message: response,
        provider,
    })
}
