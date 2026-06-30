use crate::domain::config::{AppConfig, Providers};

/// Snapshot of config fields that determine whether the cached agent must be rebuilt.
///
/// When any of these fields changes between `send_prompt` calls, `AgentManager`
/// tears down and recreates the agent with the new parameters.
#[derive(PartialEq, Eq, Clone, Debug)]
pub struct ConfigSignature {
    pub provider: Providers,
    pub api_key: String,
    pub chat_model: String,
    pub chat_base_url: String,
    pub system_prompt: String,
    pub compaction_prompt: String,
    pub compaction_threshold: usize,
    pub mcp_config_path: String,
    pub read_extensions: String,
    pub write_extensions: String,
}

impl ConfigSignature {
    pub fn from_config(config: &AppConfig) -> Self {
        let mut read_exts: Vec<&str> = config.read_extensions.iter().map(|s| s.as_str()).collect();
        read_exts.sort();
        let mut write_exts: Vec<&str> =
            config.write_extensions.iter().map(|s| s.as_str()).collect();
        write_exts.sort();

        Self {
            provider: config.provider,
            api_key: config.api_key.clone(),
            chat_model: config.chat_model.clone(),
            chat_base_url: config.chat_base_url.clone(),
            system_prompt: config.system_prompt.clone(),
            compaction_prompt: config.compaction_prompt.clone(),
            compaction_threshold: config.compaction_threshold,
            mcp_config_path: config.mcp_config_path.clone(),
            read_extensions: read_exts.join(","),
            write_extensions: write_exts.join(","),
        }
    }

    pub fn empty() -> Self {
        Self {
            provider: Providers::OpenAI,
            api_key: String::new(),
            chat_model: String::new(),
            chat_base_url: String::new(),
            system_prompt: String::new(),
            compaction_prompt: String::new(),
            compaction_threshold: 0,
            mcp_config_path: String::new(),
            read_extensions: String::new(),
            write_extensions: String::new(),
        }
    }
}
