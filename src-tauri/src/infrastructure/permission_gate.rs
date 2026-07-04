use crate::domain::errors::AppError;
use crate::domain::permission::{PermissionRequest, PermissionResponse};
use crate::infrastructure::database::PermissionRepository;
use agent_rs::agent::permission::{PermissionGate, PermissionResult};
use agent_rs::security::find_containing_root_shared;
use async_trait::async_trait;
use std::collections::HashMap;
use std::path::Path;
use std::sync::Arc;
use std::time::Duration;
use tauri::{AppHandle, Emitter};
use tokio::sync::{oneshot, Mutex};

const DEFAULT_TIMEOUT_SECS: u64 = 60;

/// In-memory cache of persisted permission preferences.
/// Key: tool_name (e.g. "write_document"). Value: all persisted entries for that tool.
#[derive(Clone)]
pub struct PrefEntry {
    pub path_pattern: Option<String>,
    pub decision: String,
}
pub type PrefCache = HashMap<String, Vec<PrefEntry>>;

/// Pending permission requests awaiting a frontend response.
/// Key: request_id (UUID). Value: oneshot sender.
type PendingMap = HashMap<String, oneshot::Sender<PermissionResponse>>;

/// Application permission gate. Implements [`PermissionGate`] from agent_rs.
///
/// On `check_permission`:
/// 1. Fast path: consult in-memory prefs cache (loaded from DB at startup).
///    If a `Allow`/`Deny` preference exists, return immediately.
/// 2. Slow path: emit a `"permission-required"` Tauri event with a [`PermissionRequest`],
///    then block on a `oneshot` channel until the frontend calls
///    `respond_to_permission` (or the timeout elapses).
///    On `AllowAlways`/`DenyAlways`, persist to DB via [`PermissionRepository`]
///    AND update the in-memory cache.
pub struct AppPermissionGate {
    pending: Mutex<PendingMap>,
    prefs: Mutex<PrefCache>,
    prefs_repo: Arc<PermissionRepository>,
    app_handle: AppHandle,
    timeout: Duration,
}

impl AppPermissionGate {
    /// Creates a new gate with an empty in-memory cache. Use [`Self::preload_preferences`]
    /// to populate the cache from the database at startup.
    pub fn new(prefs_repo: Arc<PermissionRepository>, app_handle: AppHandle) -> Self {
        Self {
            pending: Mutex::new(PendingMap::new()),
            prefs: Mutex::new(PrefCache::new()),
            prefs_repo,
            app_handle,
            timeout: Duration::from_secs(DEFAULT_TIMEOUT_SECS),
        }
    }

    /// Loads all persisted preferences into the in-memory cache.
    /// Call this at startup to seed the fast-path cache.
    pub async fn preload_preferences(&self) -> Result<(), AppError> {
        let mut cache: PrefCache = HashMap::new();
        for pref in self.prefs_repo.list_preferences().await? {
            cache.entry(pref.tool_name).or_default().push(PrefEntry {
                path_pattern: pref.path_pattern,
                decision: pref.decision,
            });
        }
        *self.prefs.lock().await = cache;
        Ok(())
    }

    /// Dispatches a frontend permission response to a pending request.
    ///
    /// Returns `Ok(())` if the request was found and the response was sent.
    /// Returns `Err(AppError::PermissionError)` if no pending request with that id exists
    /// (e.g. it already timed out, or the id is invalid).
    pub async fn resolve_permission(
        &self,
        request_id: &str,
        response: PermissionResponse,
    ) -> Result<(), AppError> {
        let sender = {
            let mut pending = self.pending.lock().await;
            pending.remove(request_id)
        };
        match sender {
            Some(tx) => tx.send(response).map_err(|_| {
                AppError::PermissionError(format!(
                    "Failed to dispatch permission response for request_id={}",
                    request_id
                ))
            }),
            None => Err(AppError::PermissionError(format!(
                "No pending permission request with id={}",
                request_id
            ))),
        }
    }

    /// Refreshes the in-memory prefs cache from the DB.
    pub async fn reload_preferences(&self) -> Result<(), AppError> {
        let mut cache: PrefCache = HashMap::new();
        for pref in self.prefs_repo.list_preferences().await? {
            cache.entry(pref.tool_name).or_default().push(PrefEntry {
                path_pattern: pref.path_pattern,
                decision: pref.decision,
            });
        }
        *self.prefs.lock().await = cache;
        Ok(())
    }

    /// Allows overriding the default timeout (mainly for tests).
    #[allow(dead_code)]
    pub fn with_timeout(mut self, timeout: Duration) -> Self {
        self.timeout = timeout;
        self
    }
}

#[async_trait]
impl PermissionGate for AppPermissionGate {
    async fn check_permission(&self, tool_name: &str, description: &str) -> PermissionResult {
        // Fast path: check if the requested path is inside the sandbox.
        // If so, auto-allow without prompting the user.
        if let Some(path_str) = extract_path_from_description(description) {
            if let Some(sandbox) = crate::infrastructure::agent::try_get_shared_sandbox() {
                let path = Path::new(&path_str);
                if find_containing_root_shared(&sandbox, path).is_some() {
                    return PermissionResult::Allow;
                }
            }
        }
        // Path is outside the sandbox (or unparseable) — fall through to slow path.

        // Fast path: consult persisted user preference (in-memory cache).
        // This honors previously-saved "Allow Always" / "Deny Always" decisions
        // so the user is not re-prompted for tools they already decided on.
        let matched: Option<String> = {
            let prefs = self.prefs.lock().await;
            match prefs.get(tool_name) {
                Some(entries) if !entries.is_empty() => {
                    let extracted_path = extract_path_from_description(description);
                    pick_best_match(entries, extracted_path.as_deref()).map(|e| e.decision.clone())
                }
                _ => None,
            }
        };
        if let Some(decision) = matched {
            match decision.as_str() {
                "allow" => return PermissionResult::Allow,
                "deny" => {
                    return PermissionResult::Deny {
                        reason: format!("Tool '{}' is denied by user preference", tool_name),
                    }
                }
                _ => {}
            }
        }

        // Slow path: emit event, await response
        let request_id = uuid::Uuid::new_v4().to_string();
        let (tx, rx) = oneshot::channel();

        {
            let mut pending = self.pending.lock().await;
            pending.insert(request_id.clone(), tx);
        }

        let request = PermissionRequest {
            request_id: request_id.clone(),
            tool_name: tool_name.to_string(),
            description: description.to_string(),
        };

        if self
            .app_handle
            .emit("permission-required", &request)
            .is_err()
        {
            let mut pending = self.pending.lock().await;
            pending.remove(&request_id);
            tracing::error!(request_id = %request_id, "Failed to emit permission-required event to frontend");
            return PermissionResult::Deny {
                reason: "Failed to emit permission request to frontend".to_string(),
            };
        }

        let response = tokio::time::timeout(self.timeout, rx).await;

        {
            let mut pending = self.pending.lock().await;
            pending.remove(&request_id);
        }

        handle_permission_response(
            &self.prefs_repo,
            &self.prefs,
            tool_name,
            description,
            response,
            self.timeout,
        )
        .await
    }
}

pub async fn handle_permission_response(
    repo: &PermissionRepository,
    prefs: &Mutex<PrefCache>,
    tool_name: &str,
    description: &str,
    response: Result<
        Result<PermissionResponse, oneshot::error::RecvError>,
        tokio::time::error::Elapsed,
    >,
    timeout: Duration,
) -> PermissionResult {
    match response {
        Ok(Ok(PermissionResponse::Allow)) => PermissionResult::Allow,
        Ok(Ok(PermissionResponse::Deny { reason })) => PermissionResult::Deny { reason },
        Ok(Ok(PermissionResponse::AllowAlways { path })) => {
            let scope = path.or_else(|| derive_scope_from_description(description));
            if let Some(s) = scope {
                if let Err(e) = repo.set_preference(tool_name, Some(&s), "allow").await {
                    tracing::warn!(error = %e, "failed to persist allow-always preference");
                }
                let mut prefs = prefs.lock().await;
                let entries = prefs.entry(tool_name.to_string()).or_default();
                if let Some(existing) = entries
                    .iter_mut()
                    .find(|e| e.path_pattern.as_ref() == Some(&s))
                {
                    existing.decision = "allow".to_string();
                } else {
                    entries.push(PrefEntry {
                        path_pattern: Some(s),
                        decision: "allow".to_string(),
                    });
                }
            } else {
                tracing::debug!(
                    tool_name,
                    "Skipping Always-Allow persistence: no path derivable from description"
                );
            }
            PermissionResult::Allow
        }
        Ok(Ok(PermissionResponse::DenyAlways { path })) => {
            let scope = path.or_else(|| derive_scope_from_description(description));
            if let Some(s) = scope {
                if let Err(e) = repo.set_preference(tool_name, Some(&s), "deny").await {
                    tracing::warn!(error = %e, "failed to persist deny-always preference");
                }
                let mut prefs = prefs.lock().await;
                let entries = prefs.entry(tool_name.to_string()).or_default();
                if let Some(existing) = entries
                    .iter_mut()
                    .find(|e| e.path_pattern.as_ref() == Some(&s))
                {
                    existing.decision = "deny".to_string();
                } else {
                    entries.push(PrefEntry {
                        path_pattern: Some(s),
                        decision: "deny".to_string(),
                    });
                }
            } else {
                tracing::debug!(
                    tool_name,
                    "Skipping Always-Deny persistence: no path derivable from description"
                );
            }
            PermissionResult::Deny {
                reason: format!("Tool '{}' is permanently denied by user", tool_name),
            }
        }
        Ok(Err(_)) => PermissionResult::Deny {
            reason: "Permission request channel was cancelled".to_string(),
        },
        Err(_) => PermissionResult::Deny {
            reason: format!("Permission request timed out after {}s", timeout.as_secs()),
        },
    }
}

/// Extracts a file path from a known agent_rs tool description prefix.
/// Returns `None` if the description doesn't match any known prefix.
pub fn extract_path_from_description(description: &str) -> Option<String> {
    const KNOWN_PREFIXES: &[&str] = &[
        "Wants to write file at ",
        "Wants to read file at ",
        "Wants to list directory ",
        "Wants to search files in ",
        "Wants to grep files in ",
    ];
    for prefix in KNOWN_PREFIXES {
        if let Some(rest) = description.strip_prefix(prefix) {
            let trimmed = rest.trim_start().trim_end_matches(['[', ']', ' ']);
            if !trimmed.is_empty() {
                return Some(trimmed.to_string());
            }
        }
    }
    None
}

/// Derives a directory scope from a tool description for persistent permission decisions.
///
/// For directory-context tools (`Wants to list directory`, `Wants to search files in`,
/// `Wants to grep files in`), returns the extracted path directly.
/// For file-context tools (`Wants to write file at`, `Wants to read file at`),
/// returns the parent directory (everything up to the last `/`). Falls back to `"/"`.
/// If no path can be extracted, returns `None` — callers should skip persistence.
pub fn derive_scope_from_description(description: &str) -> Option<String> {
    let path = extract_path_from_description(description)?;

    let is_dir_context = description.starts_with("Wants to list directory ")
        || description.starts_with("Wants to search files in ")
        || description.starts_with("Wants to grep files in ");

    if is_dir_context {
        Some(path)
    } else {
        match path.rfind('/') {
            Some(pos) if pos > 0 => Some(path[..pos].to_string()),
            _ => Some("/".to_string()),
        }
    }
}

/// Normalizes a path for prefix comparison: ensures it ends with `/`.
fn normalize_for_prefix(p: &str) -> String {
    if p.ends_with('/') {
        p.to_string()
    } else {
        format!("{}/", p)
    }
}

/// Picks the best matching entry from a list of preferences for the given extracted path.
///
/// Priority: longest matching path-specific rule > global rule > no match.
fn pick_best_match<'a>(
    entries: &'a [PrefEntry],
    extracted_path: Option<&str>,
) -> Option<&'a PrefEntry> {
    if let Some(path) = extracted_path {
        let normalized = normalize_for_prefix(path);
        let mut best: Option<&'a PrefEntry> = None;
        for entry in entries {
            if let Some(ref pattern) = entry.path_pattern {
                let normalized_pattern = normalize_for_prefix(pattern);
                if normalized.starts_with(&normalized_pattern) {
                    match best {
                        Some(b)
                            if b.path_pattern.as_ref().map(|p| p.len()).unwrap_or(0)
                                >= pattern.len() => {}
                        _ => best = Some(entry),
                    }
                }
            }
        }
        if best.is_some() {
            return best;
        }
    }
    entries.iter().find(|e| e.path_pattern.is_none())
}
