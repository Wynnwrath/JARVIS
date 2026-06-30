use jarvis_lib::domain::config::{AppConfig, Providers};
use jarvis_lib::infrastructure::agent::ConfigSignature;

fn test_config() -> AppConfig {
    let mut read_exts = std::collections::HashSet::new();
    read_exts.insert("txt".to_string());
    read_exts.insert("md".to_string());
    let mut write_exts = std::collections::HashSet::new();
    write_exts.insert("rs".to_string());
    write_exts.insert("toml".to_string());
    AppConfig {
        provider: Providers::OpenAI,
        silence_threshold_rms: 0.01,
        silence_duration_ms: 500,
        api_key: "sk-test".to_string(),
        chat_model: "gpt-4".to_string(),
        chat_base_url: String::new(),
        mcp_config_path: String::new(),
        transcription_model_path: String::new(),
        database_name: "test.db".to_string(),
        system_prompt: "You are helpful.".to_string(),
        compaction_prompt: "Compact.".to_string(),
        compaction_threshold: 100,
        sandbox_dir: ".".to_string(),
        sandbox_roots: vec![],
        read_extensions: read_exts,
        write_extensions: write_exts,
    }
}

#[test]
fn from_config_produces_deterministic_signature() {
    let config = test_config();
    let sig1 = ConfigSignature::from_config(&config);
    let sig2 = ConfigSignature::from_config(&config);
    assert_eq!(sig1, sig2);
}

#[test]
fn extension_order_insensitivity() {
    let mut config = test_config();
    config.read_extensions = ["md", "txt"].iter().map(|s| s.to_string()).collect();
    config.write_extensions = ["toml", "rs"].iter().map(|s| s.to_string()).collect();
    let sig1 = ConfigSignature::from_config(&config);

    config.read_extensions = ["txt", "md"].iter().map(|s| s.to_string()).collect();
    config.write_extensions = ["rs", "toml"].iter().map(|s| s.to_string()).collect();
    let sig2 = ConfigSignature::from_config(&config);

    assert_eq!(sig1, sig2, "extension ordering should not affect signature");
}

#[test]
fn empty_differs_from_real_config() {
    let config = test_config();
    let sig = ConfigSignature::from_config(&config);
    let empty = ConfigSignature::empty();
    assert_ne!(sig, empty, "real config signature should differ from empty");
}

#[test]
fn different_provider_produces_different_signature() {
    let mut config = test_config();
    let sig_openai = ConfigSignature::from_config(&config);

    config.provider = Providers::Gemini;
    let sig_gemini = ConfigSignature::from_config(&config);

    assert_ne!(sig_openai, sig_gemini);
}

#[test]
fn different_model_produces_different_signature() {
    let mut config = test_config();
    let sig1 = ConfigSignature::from_config(&config);

    config.chat_model = "gpt-3.5-turbo".to_string();
    let sig2 = ConfigSignature::from_config(&config);

    assert_ne!(sig1, sig2);
}
