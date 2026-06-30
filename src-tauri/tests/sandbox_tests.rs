use agent_rs_lib::domain::errors::DocumentError;
use agent_rs_lib::security::{SandboxConfig, SharedSandbox};
use jarvis_lib::domain::config::AppConfig;
use jarvis_lib::infrastructure::agent::sandbox::sync_sandbox_roots_for;
use std::sync::Arc;

struct TempDir(std::path::PathBuf);

impl TempDir {
    fn new(name: &str) -> Self {
        let dir = std::env::temp_dir().join(format!(
            "jarvis_sandbox_test_{}_{}",
            name,
            uuid::Uuid::new_v4()
        ));
        std::fs::create_dir_all(&dir).unwrap();
        TempDir(dir)
    }
    fn path(&self) -> &std::path::Path {
        &self.0
    }
}

impl Drop for TempDir {
    fn drop(&mut self) {
        let _ = std::fs::remove_dir_all(&self.0);
    }
}

fn make_config(primary: &std::path::Path, extras: Vec<String>) -> AppConfig {
    AppConfig {
        sandbox_dir: primary.to_string_lossy().into_owned(),
        sandbox_roots: extras,
        ..AppConfig::default()
    }
}

#[test]
fn sync_adds_new_root() {
    let primary = TempDir::new("primary");
    let extra = TempDir::new("extra");

    let sc = SandboxConfig::single(primary.path()).unwrap();
    let shared = Arc::new(SharedSandbox::from(sc));

    let config = make_config(primary.path(), vec![extra.path().to_string_lossy().into_owned()]);
    sync_sandbox_roots_for(&shared, &config).unwrap();

    let snapshot = shared.snapshot();
    assert_eq!(
        snapshot.canonical_roots().len(),
        2,
        "should have 2 roots after sync"
    );
    assert!(shared.contains_root(primary.path()).unwrap());
    assert!(shared.contains_root(extra.path()).unwrap());
}

#[test]
fn sync_removes_missing_root() {
    let primary = TempDir::new("primary");
    let extra = TempDir::new("extra");

    let sc = SandboxConfig::new(vec![primary.path().to_path_buf(), extra.path().to_path_buf()]).unwrap();
    let shared = Arc::new(SharedSandbox::from(sc));

    assert_eq!(shared.snapshot().canonical_roots().len(), 2);

    let config = make_config(primary.path(), vec![]);
    sync_sandbox_roots_for(&shared, &config).unwrap();

    let snapshot = shared.snapshot();
    assert_eq!(
        snapshot.canonical_roots().len(),
        1,
        "should have 1 root after removal"
    );
    assert!(shared.contains_root(primary.path()).unwrap());
    assert!(!shared.contains_root(extra.path()).unwrap());
}

#[test]
fn sync_is_idempotent() {
    let primary = TempDir::new("primary");
    let extra = TempDir::new("extra");

    let sc = SandboxConfig::single(primary.path()).unwrap();
    let shared = Arc::new(SharedSandbox::from(sc));

    let config = make_config(primary.path(), vec![extra.path().to_string_lossy().into_owned()]);

    sync_sandbox_roots_for(&shared, &config).unwrap();
    let count_before = shared.snapshot().canonical_roots().len();

    sync_sandbox_roots_for(&shared, &config).unwrap();
    let count_after = shared.snapshot().canonical_roots().len();

    assert_eq!(
        count_before, count_after,
        "idempotent sync should not change root count"
    );
}

#[test]
fn add_root_canonicalizes() {
    let primary = TempDir::new("primary");
    let sc = SandboxConfig::single(primary.path()).unwrap();
    let shared = Arc::new(SharedSandbox::from(sc));

    let result = shared.add_root("/nonexistent/path/that/does/not/exist");
    assert!(result.is_err());
    match result.unwrap_err() {
        DocumentError::Io(_) => {}
        other => panic!("expected DocumentError::Io, got {:?}", other),
    }
}

#[test]
fn remove_last_root_rejected() {
    let primary = TempDir::new("primary");
    let sc = SandboxConfig::single(primary.path()).unwrap();
    let shared = Arc::new(SharedSandbox::from(sc));

    let result = shared.remove_root(primary.path());
    assert!(result.is_err());
    match result.unwrap_err() {
        DocumentError::Sandbox(_) => {}
        other => panic!("expected DocumentError::Sandbox, got {:?}", other),
    }

    assert_eq!(
        shared.snapshot().canonical_roots().len(),
        1,
        "root count should remain 1"
    );
}
