use super::history::{
    assistant_message_text, deduplicate_consecutive_assistant_messages,
    prepare_prompt_with_attachments, update_history_with_clean_user_message,
};
use crate::domain::chat::StreamEvent;
use crate::domain::config::AppConfig;
use crate::domain::errors::AppError;
use crate::infrastructure::agent::AGENT_MANAGER;
use crate::infrastructure::database::SessionRepository;
use agent_rs::agent::memory::tokenizer;
use rig_core::message::{AssistantContent, Message, UserContent};
use std::collections::HashMap;
use std::sync::{Arc, LazyLock};

/// Per-session lock that serialises load→agent→save critical sections.
/// Prevents two concurrent `send_prompt`/`send_stream_prompt` calls for
/// the same session from racing on history reads and writes.
static SESSION_LOCKS: LazyLock<tokio::sync::Mutex<HashMap<String, Arc<tokio::sync::Mutex<()>>>>> =
    LazyLock::new(|| tokio::sync::Mutex::new(HashMap::new()));

async fn acquire_session_lock(session_id: &str) -> Arc<tokio::sync::Mutex<()>> {
    let mut map = SESSION_LOCKS.lock().await;
    map.entry(session_id.to_string())
        .or_insert_with(|| Arc::new(tokio::sync::Mutex::new(())))
        .clone()
}

/// Sends a user prompt to the cached LLM agent and persists the conversation.
///
/// 1. Loads session history from the database (propagates errors instead of
///    silently swallowing them with `unwrap_or_default`).
/// 2. Prepends attachment paths as hints for the agent's `read_document` tool.
/// 3. Calls `AGENT_MANAGER.send_prompt`, which lazily rebuilds the agent
///    when config changes and emits `"mcp-connection-error"` events via `app`.
/// 4. Cleans attachment metadata from the saved user message (replaces the
///    verbose `[Attached Document: …]` hint with a concise `[Attached: …]` marker).
/// 5. Persists the updated history back to the database.
///
/// # Arguments
///
/// * `session_id` - The unique identifier of the session to continue.
/// * `input` - The user's raw prompt text.
/// * `attachments` - Optional file paths to prepend as document hints for the agent.
/// * `config` - The current application configuration (used for agent rebuild checks).
/// * `repo` - A [`SessionRepository`] bound to the database.
/// * `app` - Optional Tauri `AppHandle`; when provided, MCP connection errors during
///   agent rebuilding emit a `"mcp-connection-error"` event to the frontend.
///
/// # Returns
///
/// Returns the agent's text reply on success, or an [`AppError`] on failure.
///
/// # Errors
///
/// Propagates any error from history retrieval, agent chat, or history persistence.
/// Returns [`AppError::SystemError`] if the agent cannot be built or the LLM call fails.
pub async fn send_prompt(
    session_id: &str,
    input: &str,
    attachments: Option<&[String]>,
    config: &AppConfig,
    repo: &SessionRepository,
    app: Option<&tauri::AppHandle>,
) -> Result<String, AppError> {
    let lock = acquire_session_lock(session_id).await;
    let _guard = lock.lock().await;

    let mut history = repo.get_session_history(session_id).await?;

    let prompt_with_attachments = prepare_prompt_with_attachments(input, attachments);

    let response = AGENT_MANAGER
        .send_prompt(&prompt_with_attachments, &mut history, config, app)
        .await?;

    update_history_with_clean_user_message(&mut history, input, attachments);

    repo.save_session_history(session_id, &history).await?;

    maybe_compact(session_id, &history, config, repo).await;

    Ok(response)
}

/// Sends a user prompt to the cached LLM agent, streams the response, and persists the conversation.
///
/// 1. Loads session history from the database.
/// 2. Prepends attachment paths as hints for the agent's `read_document` tool.
/// 3. Calls `AGENT_MANAGER.send_stream_prompt`, which streams response tokens through the channel.
/// 4. Cleans attachment metadata from the saved user message.
/// 5. Persists the updated history back to the database.
///
/// # Arguments
///
/// * `session_id` - The unique identifier of the session to continue.
/// * `input` - The user's raw prompt text.
/// * `attachments` - Optional file paths to prepend as document hints for the agent.
/// * `config` - The current application configuration.
/// * `repo` - A [`SessionRepository`] bound to the database.
/// * `app` - Optional Tauri `AppHandle`.
/// * `channel` - Tauri IPC Channel to emit streaming tokens back to the frontend.
///
/// # Returns
///
/// Returns the assistant's final response text on success, or an [`AppError`] on failure.
pub async fn send_stream_prompt(
    session_id: &str,
    input: &str,
    attachments: Option<&[String]>,
    config: &AppConfig,
    repo: &SessionRepository,
    app: Option<&tauri::AppHandle>,
    channel: tauri::ipc::Channel<StreamEvent>,
) -> Result<String, AppError> {
    let lock = acquire_session_lock(session_id).await;
    let _guard = lock.lock().await;

    let history = repo.get_session_history(session_id).await?;

    let prompt_with_attachments = prepare_prompt_with_attachments(input, attachments);

    let mut updated_history = AGENT_MANAGER
        .send_stream_prompt(&prompt_with_attachments, &history, config, app, &channel)
        .await?;

    let final_response = if let Some(Message::Assistant { content, .. }) = updated_history.last() {
        if let Some(text) = content.iter().find_map(|item| match item {
            AssistantContent::Text(t) => Some(t.text.clone()),
            _ => None,
        }) {
            text
        } else {
            return Err(AppError::SystemError(
                "Streaming response completed but did not contain any text content".to_string(),
            ));
        }
    } else {
        return Err(AppError::SystemError(
            "Streaming response completed but no assistant message was appended to history"
                .to_string(),
        ));
    };

    update_history_with_clean_user_message(&mut updated_history, input, attachments);

    let updated_history = deduplicate_consecutive_assistant_messages(updated_history);

    repo.save_session_history(session_id, &updated_history)
        .await?;

    maybe_compact(session_id, &updated_history, config, repo).await;

    Ok(final_response)
}

/// Compact session history when it exceeds the configured threshold.
///
/// Triggers asynchronously (fire-and-forget) so the chat response is not delayed.
/// When triggered, older messages are summarised into a single system message and
/// the compacted history replaces the originals.
async fn maybe_compact(
    session_id: &str,
    history: &[Message],
    config: &AppConfig,
    repo: &SessionRepository,
) {
    if count_history_tokens(history) <= config.compaction_threshold {
        return;
    }
    if history.len() <= KEEP_RECENT {
        return;
    }

    let split = history.len() - KEEP_RECENT;

    let has_system_prefix = matches!(history.first(), Some(Message::System { .. }));
    let to_summarise = if has_system_prefix && split >= 1 {
        &history[1..split]
    } else {
        &history[..split]
    };

    if to_summarise.is_empty() {
        return;
    }

    let mut summary_parts: Vec<String> = Vec::new();
    for msg in to_summarise {
        let (role, text) = match msg {
            Message::User { content } => {
                let t = content
                    .iter()
                    .filter_map(|c| match c {
                        UserContent::Text(t) => Some(t.text.as_str()),
                        _ => None,
                    })
                    .collect::<Vec<_>>()
                    .join(" ");
                ("User", t)
            }
            Message::Assistant { content, .. } => ("Assistant", assistant_message_text(content)),
            _ => continue,
        };
        if !text.is_empty() {
            summary_parts.push(format!("{}: {}", role, text));
        }
    }

    let summary_text = format!(
        "[Context compacted — summarising {} earlier messages]\n{}",
        to_summarise.len(),
        summary_parts.join("\n")
    );

    let summary_msg = Message::system(&summary_text);

    let up_to_seq = (split - 1) as i32;
    if let Err(e) = repo
        .compact_session_history(session_id, &summary_msg, up_to_seq)
        .await
    {
        tracing::warn!(error = %e, session_id, "compaction failed");
    }
}

const KEEP_RECENT: usize = 20;

fn count_history_tokens(history: &[Message]) -> usize {
    history
        .iter()
        .map(|msg| {
            let text = match msg {
                Message::User { content, .. } => content
                    .iter()
                    .filter_map(|c| match c {
                        UserContent::Text(t) => Some(t.text.as_str()),
                        _ => None,
                    })
                    .collect::<Vec<_>>()
                    .join(" "),
                Message::Assistant { content, .. } => assistant_message_text(content),
                Message::System { content, .. } => content.clone(),
            };
            tokenizer::count_string_tokens(&text)
        })
        .sum()
}
