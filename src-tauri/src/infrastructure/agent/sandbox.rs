use crate::domain::config::AppConfig;
use crate::domain::errors::AppError;
use agent_rs::security::{SandboxConfig, SharedSandbox};
use std::collections::HashSet;
use std::path::PathBuf;
use std::sync::{Arc, OnceLock};

pub(crate) static SHARED_SANDBOX: OnceLock<Arc<SharedSandbox>> = OnceLock::new();

pub fn try_get_shared_sandbox() -> Option<Arc<SharedSandbox>> {
    SHARED_SANDBOX.get().cloned()
}

#[allow(clippy::expect_used)]
pub(crate) fn get_or_init_shared_sandbox(config: &AppConfig) -> Arc<SharedSandbox> {
    SHARED_SANDBOX
        .get_or_init(|| {
            let mut roots = vec![PathBuf::from(&config.sandbox_dir)];
            roots.extend(config.sandbox_roots.iter().map(PathBuf::from));
            let sc = SandboxConfig::new(roots).expect("Initial sandbox roots are invalid");
            Arc::new(SharedSandbox::from(sc))
        })
        .clone()
}

pub fn sync_sandbox_roots(config: &AppConfig) -> Result<(), AppError> {
    if let Some(shared) = try_get_shared_sandbox() {
        sync_sandbox_roots_for(&shared, config)?;
    }
    Ok(())
}

pub fn sync_sandbox_roots_for(shared: &SharedSandbox, config: &AppConfig) -> Result<(), AppError> {
    let snapshot = shared.snapshot();
    let current_canonical: HashSet<PathBuf> = snapshot.canonical_roots().iter().cloned().collect();

    let mut desired_paths: Vec<PathBuf> = Vec::new();
    desired_paths.push(PathBuf::from(&config.sandbox_dir));
    for r in &config.sandbox_roots {
        desired_paths.push(PathBuf::from(r));
    }

    let desired_canonical: Result<HashSet<PathBuf>, AppError> = desired_paths
        .iter()
        .map(|p| {
            std::fs::canonicalize(p).map_err(|e| {
                AppError::SystemError(format!("cannot canonicalize '{}': {}", p.display(), e))
            })
        })
        .collect();
    let desired_canonical = desired_canonical?;

    let mut last_err: Option<String> = None;
    for old in &current_canonical {
        if !desired_canonical.contains(old) {
            if let Err(e) = shared.remove_root(old) {
                last_err = Some(e.to_string());
            }
        }
    }

    for new in &desired_paths {
        let canonical = std::fs::canonicalize(new).map_err(|e| {
            AppError::SystemError(format!("cannot canonicalize '{}': {}", new.display(), e))
        })?;
        if !current_canonical.contains(&canonical) {
            if let Err(e) = shared.add_root(new) {
                last_err = Some(e.to_string());
            }
        }
    }

    if let Some(e) = last_err {
        return Err(AppError::SystemError(e));
    }

    Ok(())
}
