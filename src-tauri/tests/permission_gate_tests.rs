use agent_rs::agent::permission::PermissionResult;
use jarvis_lib::domain::permission::PermissionResponse;
use jarvis_lib::infrastructure::database::{create_pool, run_migrations, PermissionRepository};
use jarvis_lib::infrastructure::permission_gate::{
    derive_scope_from_description, extract_path_from_description, handle_permission_response,
    PrefCache,
};
use std::collections::HashMap;
use std::sync::Arc;
use std::time::Duration;
use tokio::sync::Mutex;

fn unique_db_path(suffix: &str) -> std::path::PathBuf {
    let id = uuid::Uuid::new_v4();
    std::env::temp_dir().join(format!("jarvis_perm_test_{}_{}.db", suffix, id))
}

async fn setup_repo(label: &str) -> (Arc<PermissionRepository>, std::path::PathBuf) {
    let path = unique_db_path(label);
    run_migrations(path.to_str().unwrap()).expect("migrations failed");
    let pool = create_pool(path.to_str().unwrap());
    (Arc::new(PermissionRepository::new(pool)), path)
}

fn cleanup(path: &std::path::Path) {
    let _ = std::fs::remove_file(path);
    let _ = std::fs::remove_file(format!("{}-wal", path.display()));
    let _ = std::fs::remove_file(format!("{}-shm", path.display()));
}

// ── extract_path_from_description (private helper, Phase 2.2 prefix matcher) ──

#[tokio::test]
async fn extract_path_write_file() {
    let result = tokio::time::timeout(Duration::from_millis(500), async {
        extract_path_from_description("Wants to write file at /some/file.txt")
    })
    .await
    .expect("timeout");
    assert_eq!(result.as_deref(), Some("/some/file.txt"));
}

#[tokio::test]
async fn extract_path_read_file() {
    let result = tokio::time::timeout(Duration::from_millis(500), async {
        extract_path_from_description("Wants to read file at /read/me.md")
    })
    .await
    .expect("timeout");
    assert_eq!(result.as_deref(), Some("/read/me.md"));
}

#[tokio::test]
async fn extract_path_list_directory() {
    let result = tokio::time::timeout(Duration::from_millis(500), async {
        extract_path_from_description("Wants to list directory /src")
    })
    .await
    .expect("timeout");
    assert_eq!(result.as_deref(), Some("/src"));
}

#[tokio::test]
async fn extract_path_search_files() {
    let result = tokio::time::timeout(Duration::from_millis(500), async {
        extract_path_from_description("Wants to search files in /project")
    })
    .await
    .expect("timeout");
    assert_eq!(result.as_deref(), Some("/project"));
}

#[tokio::test]
async fn extract_path_grep_files() {
    let result = tokio::time::timeout(Duration::from_millis(500), async {
        extract_path_from_description("Wants to grep files in /src")
    })
    .await
    .expect("timeout");
    assert_eq!(result.as_deref(), Some("/src"));
}

#[tokio::test]
async fn extract_path_strips_trailing_brackets() {
    let result = tokio::time::timeout(Duration::from_millis(500), async {
        extract_path_from_description("Wants to write file at /some/file.txt]")
    })
    .await
    .expect("timeout");
    assert_eq!(result.as_deref(), Some("/some/file.txt"));
}

#[tokio::test]
async fn extract_path_unknown_prefix_returns_none() {
    let result = tokio::time::timeout(Duration::from_millis(500), async {
        extract_path_from_description("Some unknown description")
    })
    .await
    .expect("timeout");
    assert!(result.is_none());
}

#[tokio::test]
async fn extract_path_empty_after_prefix_returns_none() {
    let result = tokio::time::timeout(Duration::from_millis(500), async {
        extract_path_from_description("Wants to write file at ")
    })
    .await
    .expect("timeout");
    assert!(result.is_none());
}

// ── PermissionRepository: persist + retrieve (validates AllowAlways/DenyAlways DB path) ──

#[tokio::test]
async fn allow_always_persists_to_db() {
    let (repo, path) = setup_repo("allow_always").await;
    let result = tokio::time::timeout(Duration::from_millis(500), async {
        repo.set_preference("my_tool", None, "allow").await
    })
    .await
    .expect("timeout");
    assert!(result.is_ok());

    let pref = repo.get_preference("my_tool", None).await.unwrap();
    assert_eq!(pref.as_deref(), Some("allow"));
    cleanup(&path);
}

#[tokio::test]
async fn deny_always_persists_to_db() {
    let (repo, path) = setup_repo("deny_always").await;
    let result = tokio::time::timeout(Duration::from_millis(500), async {
        repo.set_preference("my_tool", None, "deny").await
    })
    .await
    .expect("timeout");
    assert!(result.is_ok());

    let pref = repo.get_preference("my_tool", None).await.unwrap();
    assert_eq!(pref.as_deref(), Some("deny"));
    cleanup(&path);
}

#[tokio::test]
async fn list_preferences_returns_all() {
    let (repo, path) = setup_repo("list_prefs").await;
    tokio::time::timeout(Duration::from_millis(500), async {
        repo.set_preference("tool_a", None, "allow").await.unwrap();
        repo.set_preference("tool_b", None, "deny").await.unwrap();
    })
    .await
    .expect("timeout");

    let prefs = repo.list_preferences().await.unwrap();
    assert_eq!(prefs.len(), 2);
    cleanup(&path);
}

#[tokio::test]
async fn overwrite_existing_preference() {
    let (repo, path) = setup_repo("overwrite").await;
    tokio::time::timeout(Duration::from_millis(500), async {
        repo.set_preference("tool_x", None, "allow").await.unwrap();
        repo.set_preference("tool_x", None, "deny").await.unwrap();
    })
    .await
    .expect("timeout");

    let pref = repo.get_preference("tool_x", None).await.unwrap();
    assert_eq!(pref.as_deref(), Some("deny"));
    cleanup(&path);
}

#[tokio::test]
async fn delete_preference() {
    let (repo, path) = setup_repo("delete").await;
    tokio::time::timeout(Duration::from_millis(500), async {
        repo.set_preference("tool_y", None, "allow").await.unwrap();
        repo.delete_preference("tool_y", None).await.unwrap();
    })
    .await
    .expect("timeout");

    let pref = repo.get_preference("tool_y", None).await.unwrap();
    assert!(pref.is_none());
    cleanup(&path);
}

// ── Cache refresh after persist ──────────────────────────────────────

#[tokio::test]
async fn allow_always_refreshes_cache_via_reload() {
    let (repo, path) = setup_repo("cache_refresh_allow").await;
    tokio::time::timeout(Duration::from_millis(500), async {
        repo.set_preference("tool_z", None, "allow").await.unwrap();

        let mut cache: HashMap<String, Vec<(Option<String>, String)>> = HashMap::new();
        assert!(!cache.contains_key("tool_z"));

        for pref in repo.list_preferences().await.unwrap() {
            cache
                .entry(pref.tool_name)
                .or_default()
                .push((pref.path_pattern, pref.decision));
        }
        assert_eq!(
            cache
                .get("tool_z")
                .and_then(|v| v.first())
                .map(|(_, d)| d.as_str()),
            Some("allow")
        );
    })
    .await
    .expect("timeout");
    cleanup(&path);
}

#[tokio::test]
async fn deny_always_refreshes_cache_via_reload() {
    let (repo, path) = setup_repo("cache_refresh_deny").await;
    tokio::time::timeout(Duration::from_millis(500), async {
        repo.set_preference("tool_w", None, "deny").await.unwrap();

        let mut cache: HashMap<String, Vec<(Option<String>, String)>> = HashMap::new();
        for pref in repo.list_preferences().await.unwrap() {
            cache
                .entry(pref.tool_name)
                .or_default()
                .push((pref.path_pattern, pref.decision));
        }
        assert_eq!(
            cache
                .get("tool_w")
                .and_then(|v| v.first())
                .map(|(_, d)| d.as_str()),
            Some("deny")
        );
    })
    .await
    .expect("timeout");
    cleanup(&path);
}

// ── Timeout path (oneshot + timeout pattern) ─────────────────────────

#[tokio::test]
async fn timeout_returns_default_when_no_response() {
    tokio::time::timeout(Duration::from_millis(500), async {
        let (_tx, rx) = tokio::sync::oneshot::channel::<PermissionResponse>();
        let result = tokio::time::timeout(Duration::from_millis(50), rx).await;
        assert!(result.is_err(), "oneshot should time out");
    })
    .await
    .expect("timeout");
}

#[tokio::test]
async fn resolved_before_timeout_returns_response() {
    tokio::time::timeout(Duration::from_millis(500), async {
        let (tx, rx) = tokio::sync::oneshot::channel::<PermissionResponse>();
        tokio::spawn(async move {
            let _ = tx.send(PermissionResponse::Allow);
        });
        let result = tokio::time::timeout(Duration::from_secs(5), rx).await;
        assert!(result.is_ok());
        assert!(matches!(result.unwrap(), Ok(PermissionResponse::Allow)));
    })
    .await
    .expect("timeout");
}

// ── Auto-allow fast path (sandbox path matching) ─────────────────────

#[tokio::test]
async fn auto_allow_sandbox_path_matching() {
    use agent_rs::security::{find_containing_root_shared, SandboxConfig, SharedSandbox};

    tokio::time::timeout(Duration::from_millis(500), async {
        let tmp = std::env::temp_dir().join("jarvis_perm_sandbox_test");
        let sub = tmp.join("subdir");
        std::fs::create_dir_all(&sub).unwrap();
        let inside_file = sub.join("file.txt");
        std::fs::write(&inside_file, "").unwrap();

        let sc = SandboxConfig::single(tmp.to_str().unwrap()).unwrap();
        let sandbox = Arc::new(SharedSandbox::from(sc));

        assert!(
            find_containing_root_shared(&sandbox, &inside_file).is_some(),
            "path inside sandbox should be matched"
        );

        let outside_path = std::path::Path::new("C:\\Windows\\System32\\config.sys");
        assert!(
            find_containing_root_shared(&sandbox, outside_path).is_none(),
            "path outside sandbox should NOT be matched"
        );

        let _ = std::fs::remove_dir_all(&tmp);
    })
    .await
    .expect("timeout");
}

#[tokio::test]
async fn extract_path_sandbox_description_matches_prefix() {
    tokio::time::timeout(Duration::from_millis(500), async {
        let desc = "Wants to write file at /tmp/jarvis_sandbox_test/sub.txt";
        let path = extract_path_from_description(desc);
        assert_eq!(path.as_deref(), Some("/tmp/jarvis_sandbox_test/sub.txt"));
    })
    .await
    .expect("timeout");
}

// ── derive_scope_from_description ──────────────────────────────────

#[test]
fn derive_scope_file_context_returns_parent_dir() {
    let result = derive_scope_from_description("Wants to write file at /home/user/docs/foo.txt");
    assert_eq!(result.as_deref(), Some("/home/user/docs"));
}

#[test]
fn derive_scope_dir_context_returns_path_directly() {
    let result = derive_scope_from_description("Wants to list directory /var/log");
    assert_eq!(result.as_deref(), Some("/var/log"));
}

#[test]
fn derive_scope_search_in_dir_context() {
    let result = derive_scope_from_description("Wants to search files in /project/src");
    assert_eq!(result.as_deref(), Some("/project/src"));
}

#[test]
fn derive_scope_grep_in_dir_context() {
    let result = derive_scope_from_description("Wants to grep files in /src");
    assert_eq!(result.as_deref(), Some("/src"));
}

#[test]
fn derive_scope_file_at_root_falls_back_to_slash() {
    let result = derive_scope_from_description("Wants to write file at /foo.txt");
    assert_eq!(result.as_deref(), Some("/"));
}

#[test]
fn derive_scope_no_path_returns_none() {
    let result = derive_scope_from_description("Custom tool");
    assert!(result.is_none());
}

#[test]
fn derive_scope_empty_description_returns_none() {
    let result = derive_scope_from_description("");
    assert!(result.is_none());
}

// ── always_allow_scopes_to_extracted_path ──────────────────────────

#[tokio::test]
async fn always_allow_scopes_to_extracted_path() {
    let (repo, path) = setup_repo("scope_extract").await;
    tokio::time::timeout(Duration::from_millis(500), async {
        let scope = derive_scope_from_description("Wants to write file at /home/user/docs/foo.txt");
        assert_eq!(scope.as_deref(), Some("/home/user/docs"));

        repo.set_preference("write_document", Some("/home/user/docs"), "allow")
            .await
            .unwrap();

        let pref = repo
            .get_preference("write_document", Some("/home/user/docs"))
            .await
            .unwrap();
        assert_eq!(pref.as_deref(), Some("allow"));
    })
    .await
    .expect("timeout");
    cleanup(&path);
}

// ── always_allow_no_path_skips_persistence ─────────────────────────

#[tokio::test]
async fn always_allow_no_path_skips_persistence() {
    let (repo, path) = setup_repo("no_path_skip").await;
    tokio::time::timeout(Duration::from_millis(500), async {
        let scope = derive_scope_from_description("Custom tool");
        assert!(scope.is_none());

        let prefs = repo.list_preferences().await.unwrap();
        assert!(
            prefs.iter().all(|p| p.tool_name != "custom_tool"),
            "no preference should be persisted for a tool with no derivable scope"
        );
    })
    .await
    .expect("timeout");
    cleanup(&path);
}

// ── always_allow_precedence_global_still_works ────────────────────

#[tokio::test]
async fn always_allow_precedence_global_still_works() {
    let (repo, path) = setup_repo("global_precedence").await;
    tokio::time::timeout(Duration::from_millis(500), async {
        repo.set_preference("write_document", None, "allow")
            .await
            .unwrap();

        let pref = repo.get_preference("write_document", None).await.unwrap();
        assert_eq!(pref.as_deref(), Some("allow"));
    })
    .await
    .expect("timeout");
    cleanup(&path);
}

// ── handle_permission_response integration tests ─────────────────────

#[tokio::test]
async fn handle_allow_always_derives_scope_and_persists() {
    let (repo, path) = setup_repo("handle_allow_always").await;
    let cache: Mutex<PrefCache> = Mutex::new(HashMap::new());
    let response: Result<
        Result<PermissionResponse, tokio::sync::oneshot::error::RecvError>,
        tokio::time::error::Elapsed,
    > = Ok(Ok(PermissionResponse::AllowAlways { path: None }));

    let result = tokio::time::timeout(Duration::from_secs(60), async {
        handle_permission_response(
            &repo,
            &cache,
            "write_document",
            "Wants to write file at /home/user/docs/foo.txt",
            response,
            Duration::from_secs(60),
        )
        .await
    })
    .await
    .expect("timeout");

    assert!(matches!(result, PermissionResult::Allow));

    let pref = repo
        .get_preference("write_document", Some("/home/user/docs"))
        .await
        .unwrap();
    assert_eq!(pref.as_deref(), Some("allow"));

    let cache = cache.lock().await;
    let entries = cache.get("write_document").unwrap();
    let entry = entries
        .iter()
        .find(|e| e.path_pattern.as_deref() == Some("/home/user/docs"))
        .unwrap();
    assert_eq!(entry.decision, "allow");

    drop(cache);
    cleanup(&path);
}

#[tokio::test]
async fn handle_allow_always_no_path_skips_persistence() {
    let (repo, path) = setup_repo("handle_allow_always_no_path").await;
    let cache: Mutex<PrefCache> = Mutex::new(HashMap::new());
    let response: Result<
        Result<PermissionResponse, tokio::sync::oneshot::error::RecvError>,
        tokio::time::error::Elapsed,
    > = Ok(Ok(PermissionResponse::AllowAlways { path: None }));

    let result = tokio::time::timeout(Duration::from_secs(60), async {
        handle_permission_response(
            &repo,
            &cache,
            "custom_tool",
            "Custom tool",
            response,
            Duration::from_secs(60),
        )
        .await
    })
    .await
    .expect("timeout");

    assert!(matches!(result, PermissionResult::Allow));

    let prefs = repo.list_preferences().await.unwrap();
    assert!(
        prefs.iter().all(|p| p.tool_name != "custom_tool"),
        "no preference should be persisted when description yields no path"
    );

    let cache = cache.lock().await;
    assert!(
        !cache.contains_key("custom_tool"),
        "cache should have no entry for custom_tool"
    );

    drop(cache);
    cleanup(&path);
}

#[tokio::test]
async fn handle_deny_always_derives_scope_and_persists() {
    let (repo, path) = setup_repo("handle_deny_always").await;
    let cache: Mutex<PrefCache> = Mutex::new(HashMap::new());
    let response: Result<
        Result<PermissionResponse, tokio::sync::oneshot::error::RecvError>,
        tokio::time::error::Elapsed,
    > = Ok(Ok(PermissionResponse::DenyAlways { path: None }));

    let result = tokio::time::timeout(Duration::from_secs(60), async {
        handle_permission_response(
            &repo,
            &cache,
            "read_file",
            "Wants to read file at /etc/passwd",
            response,
            Duration::from_secs(60),
        )
        .await
    })
    .await
    .expect("timeout");

    assert!(matches!(result, PermissionResult::Deny { .. }));

    let pref = repo
        .get_preference("read_file", Some("/etc"))
        .await
        .unwrap();
    assert_eq!(pref.as_deref(), Some("deny"));

    let cache = cache.lock().await;
    let entries = cache.get("read_file").unwrap();
    let entry = entries
        .iter()
        .find(|e| e.path_pattern.as_deref() == Some("/etc"))
        .unwrap();
    assert_eq!(entry.decision, "deny");

    drop(cache);
    cleanup(&path);
}
