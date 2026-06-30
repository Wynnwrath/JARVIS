use crate::domain::chat::StreamEvent;
use crate::domain::config::AppConfig;
use crate::domain::errors::AppError;
use crate::infrastructure::permission_gate::AppPermissionGate;
use std::sync::Arc;
use tauri::Manager;
use tokio::sync::RwLock;

use super::builder::build_agent;
use super::dispatch::AppAgent;
use super::sandbox::sync_sandbox_roots;
use super::signature::ConfigSignature;

/// Thread-safe singleton that lazily builds and caches the LLM agent.
///
/// The agent is rebuilt when the config fields (provider, model, prompt, etc.) change.
/// Rebuilding uses a double-checked lock so concurrent callers never build twice.
pub struct AgentManager {
    pub agent: RwLock<Option<AppAgent>>,
    pub signature: RwLock<ConfigSignature>,
}

/// Global lazy-initialised agent manager shared across the application.
pub static AGENT_MANAGER: std::sync::LazyLock<AgentManager> =
    std::sync::LazyLock::new(|| AgentManager {
        agent: RwLock::new(None),
        signature: RwLock::new(ConfigSignature::empty()),
    });

impl AgentManager {
    /// Sends a prompt to the cached agent, rebuilding it first if config has changed.
    ///
    /// Computes a config-signature from the provided `config` and compares it with
    /// the cached signature. If they differ or no agent exists yet, the agent is
    /// rebuilt asynchronously (including MCP tool connectors). A double-checked lock
    /// prevents concurrent callers from building the agent twice.
    ///
    /// # Arguments
    ///
    /// * `prompt` - The user's input text to send to the agent.
    /// * `history` - Mutable conversation history (prompt + response appended by the agent).
    /// * `config` - The current application configuration used to compute the rebuild signature.
    /// * `app` - Optional Tauri `AppHandle`; when provided, MCP connection errors during
    ///   agent rebuild emit a `"mcp-connection-error"` Tauri event to the frontend.
    ///
    /// # Returns
    ///
    /// Returns the agent's text reply on success, or an [`AppError`] on failure.
    ///
    /// # Errors
    ///
    /// Returns [`AppError::SystemError`] if agent initialisation fails or the inner agent
    /// returns an error during chat.
    pub async fn send_prompt(
        &self,
        prompt: &str,
        history: &mut Vec<rig_core::message::Message>,
        config: &AppConfig,
        app: Option<&tauri::AppHandle>,
    ) -> Result<String, AppError> {
        sync_sandbox_roots(config)?;
        if let Some(handle) = app {
            self.build_and_store(config, handle).await?;
        }

        let agent_guard = self.agent.read().await;
        let agent = agent_guard
            .as_ref()
            .ok_or_else(|| AppError::SystemError("Agent failed to initialize".to_string()))?;

        agent.chat(prompt, history).await
    }

    /// Clears the cached agent and resets the config signature so the next
    /// prompt rebuilds the agent from the current configuration.
    pub async fn restart(&self) {
        let mut sig_guard = self.signature.write().await;
        let mut agent_guard = self.agent.write().await;
        *agent_guard = None;
        *sig_guard = ConfigSignature::empty();
    }

    /// Builds a new agent from `config` and stores it, updating the cached signature.
    ///
    /// This is the single source of truth for agent construction + storage.
    /// Callers (lazy rebuild in `send_prompt`/`send_stream_prompt` and the
    /// background prebuild) all funnel through this method.
    pub async fn build_and_store(
        &self,
        config: &AppConfig,
        app: &tauri::AppHandle,
    ) -> Result<(), AppError> {
        let gate: Option<Arc<AppPermissionGate>> = app
            .try_state::<Arc<AppPermissionGate>>()
            .map(|s| s.inner().clone());
        if gate.is_none() {
            tracing::warn!("AppPermissionGate not found in managed state during agent build — permission prompts will not be shown");
        }

        let needs_rebuild = {
            let sig_guard = self.signature.read().await;
            let agent_guard = self.agent.read().await;
            *sig_guard != ConfigSignature::from_config(config) || agent_guard.is_none()
        };

        if needs_rebuild {
            let new_agent = build_agent(config, Some(app), gate).await?;

            let mut sig_guard = self.signature.write().await;
            let mut agent_guard = self.agent.write().await;

            let fresh_sig = ConfigSignature::from_config(config);
            if *sig_guard != fresh_sig || agent_guard.is_none() {
                *agent_guard = Some(new_agent);
                *sig_guard = fresh_sig;
            }
        }

        Ok(())
    }

    /// Returns a clone of the current config signature. For testing only.
    pub async fn signature_snapshot(&self) -> ConfigSignature {
        self.signature.read().await.clone()
    }

    /// Returns `true` if a rebuild would be triggered for the given config. For testing only.
    pub async fn needs_rebuild_check(&self, config: &AppConfig) -> bool {
        let sig_guard = self.signature.read().await;
        let agent_guard = self.agent.read().await;
        *sig_guard != ConfigSignature::from_config(config) || agent_guard.is_none()
    }

    /// Streams a prompt to the cached agent, rebuilding it first if config has changed.
    ///
    /// Computes a config-signature from the provided `config` and compares it with
    /// the cached signature. If they differ or no agent exists yet, the agent is
    /// rebuilt asynchronously (including MCP tool connectors). A double-checked lock
    /// prevents concurrent callers from building the agent twice.
    ///
    /// # Arguments
    ///
    /// * `prompt` - The user's input text to send to the agent.
    /// * `history` - Slice of conversation history.
    /// * `config` - The current application configuration used to compute the rebuild signature.
    /// * `app` - Optional Tauri `AppHandle`; when provided, MCP connection errors during
    ///   agent rebuild emit a `"mcp-connection-error"` Tauri event to the frontend.
    /// * `channel` - Tauri IPC Channel to emit streaming tokens back to the frontend.
    ///
    /// # Returns
    ///
    /// Returns the updated conversation history on success, or an [`AppError`] on failure.
    pub async fn send_stream_prompt(
        &self,
        prompt: &str,
        history: &[rig_core::message::Message],
        config: &AppConfig,
        app: Option<&tauri::AppHandle>,
        channel: &tauri::ipc::Channel<StreamEvent>,
    ) -> Result<Vec<rig_core::message::Message>, AppError> {
        sync_sandbox_roots(config)?;
        if let Some(handle) = app {
            self.build_and_store(config, handle).await?;
        }

        let agent_guard = self.agent.read().await;
        let agent = agent_guard
            .as_ref()
            .ok_or_else(|| AppError::SystemError("Agent failed to initialize".to_string()))?;

        agent.stream_chat(prompt, history, channel).await
    }
}
