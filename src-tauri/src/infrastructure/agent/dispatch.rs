use crate::domain::chat::StreamEvent;
use crate::domain::errors::AppError;
use agent_rs::agent::memory::ContextManager;
use agent_rs::agent::strip_reasoning_from_history;
use agent_rs::agent::ReActExt;
use rig_core::agent::Agent;
use rig_core::completion::{CompletionModel, Prompt};
use rig_core::message::Message;
use rig_core::streaming::StreamingChat;
use rig_core::wasm_compat::{WasmCompatSend, WasmCompatSync};

use super::stream_consumer::consume_chat_stream;

/// A type-erased LLM agent that wraps any supported provider.
///
/// Each variant holds the cached rig `Agent<M>` (preamble + tools + MCP),
/// a separate compaction-model `Agent<M>`, and ReAct/compaction tuning
/// scalars. A `BuiltReAct` is constructed per non-streaming request;
/// streaming uses the rig `Agent::stream_chat()` directly.
pub enum AppAgent {
    OpenAi(AppAgentInner<rig_core::providers::openai::completion::CompletionModel>),
    Gemini(AppAgentInner<rig_core::providers::gemini::CompletionModel>),
    Anthropic(AppAgentInner<rig_core::providers::anthropic::completion::CompletionModel>),
}

/// Cached per-provider rig agent + compaction model + ReAct tuning.
///
/// The rig `Agent<M, P>` defaults to `P = ()` (which implements `PromptHook<M>`
/// for any `M: CompletionModel`). The agent is Arc-backed, so cloning for
/// `BuiltReAct` construction is cheap.
pub struct AppAgentInner<M>
where
    M: CompletionModel + WasmCompatSend + WasmCompatSync + 'static,
{
    /// Main rig agent: system preamble + tools + MCP connectors.
    pub agent: Agent<M>,
    /// Rig agent built with the compaction prompt as preamble.
    pub compaction_agent: Agent<M>,
    /// ReAct cycle cap (replaces `default_max_turns` for the ReAct loop).
    pub max_cycles: usize,
    /// Compaction threshold in tokens. Must be > 0.
    pub compaction_threshold: usize,
}

impl AppAgent {
    /// Sends a prompt to the underlying agent via ReAct with compaction.
    ///
    /// Builds a `BuiltReAct` from the cached rig `Agent`, calls
    /// `chat_compact(prompt, history)`, which handles compaction + ReAct loop
    /// + history writeback (caller-owned history — 0.7.0 API).
    pub async fn chat(&self, prompt: &str, history: &mut Vec<Message>) -> Result<String, AppError> {
        match self {
            AppAgent::OpenAi(inner) => inner.chat(prompt, history).await,
            AppAgent::Gemini(inner) => inner.chat(prompt, history).await,
            AppAgent::Anthropic(inner) => inner.chat(prompt, history).await,
        }
    }

    /// Streams a prompt to the underlying agent, calling the callback for each chunk.
    ///
    /// Uses rig's `Agent::stream_chat()` directly (bypassing agent_rs 0.7.0's
    /// broken streaming wrappers which have an unsatisfiable `M: StreamingChat`
    /// bound). Manual compaction via `ContextManager` runs before streaming.
    pub async fn stream_chat(
        &self,
        prompt: &str,
        history: &[Message],
        channel: &tauri::ipc::Channel<StreamEvent>,
    ) -> Result<Vec<Message>, AppError> {
        match self {
            AppAgent::OpenAi(inner) => inner.stream_chat(prompt, history, channel).await,
            AppAgent::Gemini(inner) => inner.stream_chat(prompt, history, channel).await,
            AppAgent::Anthropic(inner) => inner.stream_chat(prompt, history, channel).await,
        }
    }
}

impl<M> AppAgentInner<M>
where
    M: CompletionModel + WasmCompatSend + WasmCompatSync + 'static,
    Agent<M>: Prompt + Clone,
{
    /// Non-streaming chat via ReAct with compaction.
    ///
    /// Builds a `BuiltReAct<M, (), Agent<M>>` from the cached rig `Agent`,
    /// calls `chat_compact(prompt, history)`. The 0.7.0 API takes
    /// `&mut Vec<Message>` directly — no `std::mem::take`, no `react.history()`,
    /// no restore pattern. `chat_compact` handles compaction + ReAct loop +
    /// history writeback internally.
    pub async fn chat(&self, prompt: &str, history: &mut Vec<Message>) -> Result<String, AppError> {
        let react = self
            .agent
            .react()
            .max_cycles(self.max_cycles)
            .with_compaction()
            .threshold(self.compaction_threshold)
            .compaction_model(self.compaction_agent.clone())
            .build();

        react
            .chat_compact(prompt, history)
            .await
            .map_err(|e| AppError::SystemError(e.to_string()))
    }
}

impl<M> AppAgentInner<M>
where
    M: CompletionModel + WasmCompatSend + WasmCompatSync + 'static,
    M::StreamingResponse: rig_core::completion::GetTokenUsage + Send + 'static,
    Agent<M>: StreamingChat<M, M::StreamingResponse, Hook = ()> + Prompt + Clone,
{
    /// Streaming chat via direct rig `Agent::stream_chat()`.
    ///
    /// Bypasses agent_rs 0.7.0's `BuiltReAct::stream_chat_compact` which has
    /// an unsatisfiable `M: StreamingChat` bound (the impl is on `Agent<M,P>`,
    /// not on `M`). Instead, calls rig's `Agent::stream_chat()` directly —
    /// the same `MultiTurnStreamItem` stream as the old `ContextManagedAgent`.
    ///
    /// Manual compaction via `ContextManager` runs before streaming to keep
    /// the context window manageable (matching the old in-agent compaction).
    pub async fn stream_chat(
        &self,
        prompt: &str,
        history: &[Message],
        channel: &tauri::ipc::Channel<StreamEvent>,
    ) -> Result<Vec<Message>, AppError> {
        let mut working_history = history.to_vec();

        // Manual compaction (replaces old ContextManagedAgent in-agent compaction)
        let ctx = ContextManager::new(self.compaction_threshold, self.compaction_agent.clone());
        ctx.compact_history_if_needed(&mut working_history, prompt)
            .await
            .map_err(|e| AppError::SystemError(e.to_string()))?;

        // Direct rig streaming — same MultiTurnStreamItem stream as before.
        // Agent<M> implements StreamingChat<M, M::StreamingResponse> (rig-core
        // completion.rs:446), so .stream_chat() is available.
        let stream = self
            .agent
            .stream_chat(prompt, working_history.clone())
            .multi_turn(self.max_cycles)
            .await;

        let updated_history = consume_chat_stream(stream, channel).await?;
        // FinalResponse.history() returns only the new turn's messages (prompt + assistant
        // replies + tool calls), not the prior chat_history. Prepend working_history so the
        // caller receives the full conversation, matching save_session_history's skip(max_seq+1)
        // expectation and the old ContextManagedAgent oneshot behavior.
        let mut full_history = working_history;
        full_history.extend(updated_history);
        Ok(strip_reasoning_from_history(full_history))
    }
}
