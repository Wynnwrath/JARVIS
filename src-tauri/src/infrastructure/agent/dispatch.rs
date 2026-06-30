use crate::domain::chat::StreamEvent;
use crate::domain::errors::AppError;
use rig_core::agent::Agent;

use super::stream_consumer::consume_chat_stream;

/// A type-erased LLM agent that wraps any supported provider.
///
/// Each variant holds a [`ContextManagedAgent`] with the corresponding provider's
/// completion model and agent types. The outer enum dispatches `chat()` calls
/// to the inner agent without requiring the caller to know which provider is active.
pub enum AppAgent {
    /// OpenAI-compatible agent (works with local servers via custom base URL).
    OpenAi(
        agent_rs_lib::agent::agents::ContextManagedAgent<
            rig_core::providers::openai::completion::CompletionModel,
            Agent<rig_core::providers::openai::completion::CompletionModel>,
        >,
    ),
    /// Google Gemini agent.
    Gemini(
        agent_rs_lib::agent::agents::ContextManagedAgent<
            rig_core::providers::gemini::CompletionModel,
            Agent<rig_core::providers::gemini::CompletionModel>,
        >,
    ),
    /// Anthropic (Claude) agent.
    Anthropic(
        agent_rs_lib::agent::agents::ContextManagedAgent<
            rig_core::providers::anthropic::completion::CompletionModel,
            Agent<rig_core::providers::anthropic::completion::CompletionModel>,
        >,
    ),
}

impl AppAgent {
    /// Sends a prompt to the underlying agent, forwarding the conversation history.
    ///
    /// Dispatches to the appropriate provider-specific agent variant.
    ///
    /// # Arguments
    ///
    /// * `prompt` - The input text to send to the agent.
    /// * `history` - The mutable conversation history; the prompt and response
    ///   are appended by the agent internally.
    ///
    /// # Returns
    ///
    /// Returns the agent's text response on success, or an [`AppError`] on failure.
    ///
    /// # Errors
    ///
    /// Returns [`AppError::SystemError`] if the agent's chat method fails.
    pub async fn chat(
        &self,
        prompt: &str,
        history: &mut Vec<rig_core::message::Message>,
    ) -> Result<String, AppError> {
        match self {
            AppAgent::OpenAi(agent) => agent
                .chat(prompt, history)
                .await
                .map_err(|e| AppError::SystemError(e.to_string())),
            AppAgent::Gemini(agent) => agent
                .chat(prompt, history)
                .await
                .map_err(|e| AppError::SystemError(e.to_string())),
            AppAgent::Anthropic(agent) => agent
                .chat(prompt, history)
                .await
                .map_err(|e| AppError::SystemError(e.to_string())),
        }
    }

    /// Streams a prompt to the underlying agent, calling the callback for each token chunk.
    ///
    /// # Arguments
    ///
    /// * `prompt` - The input text to send to the agent.
    /// * `history` - The slice of conversation history.
    /// * `channel` - Tauri IPC Channel to emit streaming tokens back to the frontend.
    ///
    /// # Returns
    ///
    /// Returns the updated conversation history on success, or an [`AppError`] on failure.
    pub async fn stream_chat(
        &self,
        prompt: &str,
        history: &[rig_core::message::Message],
        channel: &tauri::ipc::Channel<StreamEvent>,
    ) -> Result<Vec<rig_core::message::Message>, AppError> {
        match self {
            AppAgent::OpenAi(agent) => {
                let (stream, rx) = agent
                    .stream_chat(prompt, history)
                    .await
                    .map_err(|e| AppError::SystemError(e.to_string()))?;
                consume_chat_stream(stream, rx, channel).await
            }
            AppAgent::Gemini(agent) => {
                let (stream, rx) = agent
                    .stream_chat(prompt, history)
                    .await
                    .map_err(|e| AppError::SystemError(e.to_string()))?;
                consume_chat_stream(stream, rx, channel).await
            }
            AppAgent::Anthropic(agent) => {
                let (stream, rx) = agent
                    .stream_chat(prompt, history)
                    .await
                    .map_err(|e| AppError::SystemError(e.to_string()))?;
                consume_chat_stream(stream, rx, channel).await
            }
        }
    }
}
