use jarvis_lib::domain::config::{AppConfig, Providers};
use jarvis_lib::infrastructure::agent::{AgentManager, ConfigSignature};
use std::sync::Arc;
use std::time::Duration;
use tokio::sync::RwLock;

fn config_a() -> AppConfig {
    let mut read_exts = std::collections::HashSet::new();
    read_exts.insert("txt".to_string());
    let mut write_exts = std::collections::HashSet::new();
    write_exts.insert("md".to_string());
    AppConfig {
        provider: Providers::OpenAI,
        api_key: "sk-test-a".to_string(),
        chat_model: "gpt-4".to_string(),
        chat_base_url: "http://localhost:1234/v1".to_string(),
        mcp_config_path: String::new(),
        transcription_model_path: String::new(),
        database_name: "test.db".to_string(),
        system_prompt: "You are helpful.".to_string(),
        compaction_prompt: "Compact.".to_string(),
        compaction_threshold: 100,
        sandbox_dir: ".".to_string(),
        sandbox_roots: vec![],
        silence_threshold_rms: 0.01,
        silence_duration_ms: 500,
        read_extensions: read_exts,
        write_extensions: write_exts,
    }
}

fn config_b() -> AppConfig {
    let mut read_exts = std::collections::HashSet::new();
    read_exts.insert("rs".to_string());
    let mut write_exts = std::collections::HashSet::new();
    write_exts.insert("toml".to_string());
    AppConfig {
        provider: Providers::Gemini,
        api_key: "sk-test-b".to_string(),
        chat_model: "gemini-pro".to_string(),
        chat_base_url: String::new(),
        mcp_config_path: String::new(),
        transcription_model_path: String::new(),
        database_name: "test.db".to_string(),
        system_prompt: "You are helpful.".to_string(),
        compaction_prompt: "Compact.".to_string(),
        compaction_threshold: 200,
        sandbox_dir: ".".to_string(),
        sandbox_roots: vec![],
        silence_threshold_rms: 0.01,
        silence_duration_ms: 500,
        read_extensions: read_exts,
        write_extensions: write_exts,
    }
}

fn fresh_manager() -> Arc<AgentManager> {
    Arc::new(AgentManager {
        agent: RwLock::new(None),
        signature: RwLock::new(ConfigSignature::empty()),
    })
}

#[tokio::test]
async fn concurrent_needs_rebuild_checks_do_not_deadlock() {
    let result = tokio::time::timeout(Duration::from_secs(5), async {
        let mgr = fresh_manager();
        let cfg_a = config_a();
        let cfg_b = config_b();

        let (ra, rb) = tokio::join!(
            tokio::spawn({
                let mgr = Arc::clone(&mgr);
                let cfg = cfg_a.clone();
                async move { mgr.needs_rebuild_check(&cfg).await }
            }),
            tokio::spawn({
                let mgr = Arc::clone(&mgr);
                let cfg = cfg_b.clone();
                async move { mgr.needs_rebuild_check(&cfg).await }
            }),
        );

        let res_a = ra.expect("task a panicked");
        let res_b = rb.expect("task b panicked");

        assert_eq!(
            res_a, res_b,
            "both configs should yield same needs_rebuild for fresh manager"
        );
        assert!(res_b, "fresh manager (no agent) should need rebuild");
    })
    .await;
    assert!(
        result.is_ok(),
        "concurrent needs_rebuild checks deadlocked or timed out"
    );
}

#[tokio::test]
async fn concurrent_signature_reads_do_not_deadlock() {
    let result = tokio::time::timeout(Duration::from_secs(5), async {
        let mgr = fresh_manager();
        let mut handles = vec![];
        for _ in 0..8 {
            let m = Arc::clone(&mgr);
            handles.push(tokio::spawn(async move { m.signature_snapshot().await }));
        }
        for h in handles {
            let sig = h.await.expect("task panicked");
            assert_eq!(sig, ConfigSignature::empty());
        }
    })
    .await;
    assert!(
        result.is_ok(),
        "concurrent signature reads deadlocked or timed out"
    );
}

#[tokio::test]
async fn signature_remains_consistent_after_rapid_needs_rebuild_cycle() {
    let result = tokio::time::timeout(Duration::from_secs(5), async {
        let mgr = fresh_manager();
        let cfg_a = config_a();

        for _ in 0..20 {
            let _ = mgr.needs_rebuild_check(&cfg_a).await;
        }

        let sig = mgr.signature_snapshot().await;
        assert_eq!(
            sig,
            ConfigSignature::empty(),
            "signature should still be empty (no build_and_store called)"
        );
    })
    .await;
    assert!(
        result.is_ok(),
        "rapid needs_rebuild cycle deadlocked or timed out"
    );
}

#[tokio::test]
async fn concurrent_signature_read_write_no_torn_value() {
    let result = tokio::time::timeout(Duration::from_secs(5), async {
        let mgr = fresh_manager();
        let cfg_a = config_a();
        let sig_new = ConfigSignature::from_config(&cfg_a);

        let writer = {
            let mgr = Arc::clone(&mgr);
            let sig = sig_new.clone();
            tokio::spawn(async move {
                let mut sig_guard = mgr.signature.write().await;
                *sig_guard = sig;
            })
        };

        let mut handles = vec![];
        for _ in 0..8 {
            let m = Arc::clone(&mgr);
            let expected = sig_new.clone();
            handles.push(tokio::spawn(async move {
                let sig = m.signature_snapshot().await;
                assert!(
                    sig == ConfigSignature::empty() || sig == expected,
                    "signature should be either empty or the new value, got: {:?}",
                    sig
                );
            }));
        }

        writer.await.expect("writer panicked");
        for h in handles {
            h.await.expect("reader panicked");
        }
    })
    .await;
    assert!(
        result.is_ok(),
        "concurrent read/write deadlocked or timed out"
    );
}
