use std::collections::HashSet;

pub(crate) fn default_transcription_model_path() -> String {
    "parakeet-tdt-0.6b-v3-int8".to_string()
}
pub(crate) fn default_database_name() -> String {
    "jarvis.db".to_string()
}
pub(crate) fn default_system_prompt() -> String {
    include_str!("../defaults/system_prompt.md").to_string()
}
pub(crate) fn default_compaction_prompt() -> String {
    "Summarize this context briefly, capturing key points.".to_string()
}
pub(crate) fn default_compaction_threshold() -> usize {
    32000
}
pub(crate) fn default_sandbox_dir() -> String {
    ".".to_string()
}

pub(crate) fn default_read_extensions() -> HashSet<String> {
    [
        "txt", "md", "pdf", "json", "toml", "rs", "js", "ts", "tsx", "html", "css",
    ]
    .iter()
    .map(|s| s.to_string())
    .collect()
}

pub(crate) fn default_write_extensions() -> HashSet<String> {
    [
        "txt", "md", "json", "toml", "rs", "js", "ts", "tsx", "html", "css",
    ]
    .iter()
    .map(|s| s.to_string())
    .collect()
}
