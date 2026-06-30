use crate::domain::config::{AppConfig, Providers};
use crate::domain::errors::AppError;
use crate::infrastructure::permission_gate::AppPermissionGate;
use agent_rs_lib::agent::permission::PermissionPolicy;
use agent_rs_lib::agent::tools::{
    GlobSearchTool, GrepSearchTool, ListDirectoryTool, ReadDocumentTool, WriteDocumentTool,
};
use agent_rs_lib::agent::AgentContextExt;
use agent_rs_lib::config::McpConfig;
use agent_rs_lib::mcp::client::McpClient;
use rig_core::prelude::*;
use rig_core::tool::ToolDyn;
use std::path::Path;
use std::sync::Arc;

use super::dispatch::AppAgent;
use super::sandbox::get_or_init_shared_sandbox;

pub(crate) async fn build_agent(
    config: &AppConfig,
    app: Option<&tauri::AppHandle>,
    gate: Option<Arc<AppPermissionGate>>,
) -> Result<AppAgent, AppError> {
    use tauri::Emitter;

    // Setup document tools with sandbox and extension config
    let ask_user: PermissionPolicy = if let Some(g) = gate {
        PermissionPolicy::Custom(g)
    } else {
        tracing::error!("AppPermissionGate not found in managed state — denying all tool permissions");
        PermissionPolicy::DenyAll
    };
    let sandbox = get_or_init_shared_sandbox(config);
    let read_exts = config.read_extensions.clone();
    let write_exts = config.write_extensions.clone();

    let mut tools: Vec<Box<dyn ToolDyn>> = vec![
        Box::new(ReadDocumentTool::new(
            Arc::clone(&sandbox),
            read_exts.clone(),
            ask_user.clone(),
        )),
        Box::new(WriteDocumentTool::new(
            Arc::clone(&sandbox),
            write_exts,
            ask_user.clone(),
        )),
        Box::new(ListDirectoryTool::new(
            Arc::clone(&sandbox),
            ask_user.clone(),
        )),
        Box::new(GlobSearchTool::new(Arc::clone(&sandbox), ask_user.clone())),
        Box::new(GrepSearchTool::new(
            Arc::clone(&sandbox),
            config.read_extensions.clone(),
            ask_user.clone(),
        )),
    ];

    if Path::new(&config.mcp_config_path).exists() {
        if let Ok(mcp_config) = McpConfig::from_path(&config.mcp_config_path) {
            for (name, server_def) in mcp_config.mcp_servers {
                let mut single_servers = std::collections::HashMap::new();
                single_servers.insert(name.clone(), server_def);
                let single_config = McpConfig {
                    mcp_servers: single_servers,
                };
                match McpClient::new(single_config).tools(ask_user.clone()).await {
                    Ok(mcp_tools) => {
                        tools.extend(mcp_tools);
                    }
                    Err(e) => {
                        tracing::warn!(error = %e, server = %name, "MCP server connection failed");
                        if let Some(handle) = app {
                            let _ = handle.emit(
                                "mcp-connection-error",
                                serde_json::json!({
                                    "server": name,
                                    "error": e.to_string(),
                                }),
                            );
                        }
                    }
                }
            }
        }
    }

    match config.provider {
        Providers::OpenAI => {
            let api_key = if config.api_key.is_empty() {
                "sk-local"
            } else {
                &config.api_key
            };
            let mut builder = rig_core::providers::openai::Client::builder().api_key(api_key);
            if !config.chat_base_url.is_empty() {
                builder = builder.base_url(&config.chat_base_url);
            }
            let client = builder
                .build()
                .map_err(|e| AppError::SystemError(e.to_string()))?
                .completions_api();

            let model = client.completion_model(&config.chat_model);

            let compaction_model = rig_core::agent::AgentBuilder::new(model.clone())
                .preamble(&config.compaction_prompt)
                .build();

            let agent = rig_core::agent::AgentBuilder::new(model)
                .tools(tools)
                .preamble(&config.system_prompt)
                .default_max_turns(20)
                .build()
                .with_compaction(config.compaction_threshold, compaction_model);

            Ok(AppAgent::OpenAi(agent))
        }
        Providers::Gemini => {
            let mut builder =
                rig_core::providers::gemini::Client::builder().api_key(&config.api_key);
            if !config.chat_base_url.is_empty() {
                builder = builder.base_url(&config.chat_base_url);
            }
            let client = builder
                .build()
                .map_err(|e| AppError::SystemError(e.to_string()))?;

            let compaction_model = client
                .agent(&config.chat_model)
                .preamble(&config.compaction_prompt)
                .build();

            let agent = client
                .agent(&config.chat_model)
                .tools(tools)
                .preamble(&config.system_prompt)
                .default_max_turns(20)
                .build()
                .with_compaction(config.compaction_threshold, compaction_model);

            Ok(AppAgent::Gemini(agent))
        }
        Providers::Anthropic => {
            let mut builder =
                rig_core::providers::anthropic::Client::builder().api_key(&config.api_key);
            if !config.chat_base_url.is_empty() {
                builder = builder.base_url(&config.chat_base_url);
            }
            let client = builder
                .build()
                .map_err(|e| AppError::SystemError(e.to_string()))?;

            let compaction_model = client
                .agent(&config.chat_model)
                .preamble(&config.compaction_prompt)
                .build();

            let agent = client
                .agent(&config.chat_model)
                .tools(tools)
                .preamble(&config.system_prompt)
                .default_max_turns(20)
                .build()
                .with_compaction(config.compaction_threshold, compaction_model);

            Ok(AppAgent::Anthropic(agent))
        }
    }
}

/// Prebuilds the agent in the background and stores it in [`AGENT_MANAGER`].
///
/// Emits `"agent-status"` Tauri events so the frontend can display progress:
/// - `{"status": "building"}` — build started
/// - `{"status": "ready", "provider": "<Debug-formatted provider>"}` — build succeeded
/// - `{"status": "error", "error": "<message>"}` — build failed
pub async fn prebuild_agent(app: tauri::AppHandle) {
    use tauri::{Emitter, Manager};

    let _ = app.emit("agent-status", serde_json::json!({ "status": "building" }));

    let config = {
        let state = app.state::<tokio::sync::RwLock<AppConfig>>();
        let guard = state.read().await;
        guard.clone()
    };

    match super::manager::AGENT_MANAGER
        .build_and_store(&config, &app)
        .await
    {
        Ok(()) => {
            let _ = app.emit(
                "agent-status",
                serde_json::json!({
                    "status": "ready",
                    "provider": format!("{:?}", config.provider),
                }),
            );
        }
        Err(e) => {
            let _ = app.emit(
                "agent-status",
                serde_json::json!({
                    "status": "error",
                    "error": e.to_string(),
                }),
            );
        }
    }
}
