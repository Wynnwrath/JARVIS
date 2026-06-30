use crate::domain::errors::AppError;
use crate::infrastructure::agent::try_get_shared_sandbox;
use crate::infrastructure::permission_gate::AppPermissionGate;
use agent_rs_lib::agent::permission::PermissionPolicy;
use agent_rs_lib::agent::tools::{
    GlobSearchTool, GrepSearchTool, ListDirectoryTool, ReadDocumentTool, WriteDocumentTool,
};
use rig_core::tool::ToolDyn;
use std::collections::HashSet;
use std::sync::Arc;

pub async fn read_document(
    allowed_extensions: HashSet<String>,
    path: String,
    gate: Option<Arc<AppPermissionGate>>,
) -> Result<String, AppError> {
    let sandbox = try_get_shared_sandbox()
        .ok_or_else(|| AppError::SystemError("sandbox not initialized".into()))?;
    let policy = gate
        .map(|g| {
            PermissionPolicy::Custom(
                g as Arc<dyn agent_rs_lib::agent::permission::PermissionGate + Send + Sync>,
            )
        })
        .unwrap_or_else(|| {
            tracing::error!("AppPermissionGate not found in managed state for read_document — denying access");
            PermissionPolicy::DenyAll
        });
    let tool = ReadDocumentTool::new(sandbox, allowed_extensions, policy);
    let args = serde_json::json!({ "path": path }).to_string();
    tool.call(args)
        .await
        .map_err(|e| AppError::SystemError(e.to_string()))
}

pub async fn write_document(
    allowed_extensions: HashSet<String>,
    path: String,
    content: String,
    append: Option<bool>,
    gate: Option<Arc<AppPermissionGate>>,
) -> Result<String, AppError> {
    let sandbox = try_get_shared_sandbox()
        .ok_or_else(|| AppError::SystemError("sandbox not initialized".into()))?;
    let policy = gate
        .map(|g| {
            PermissionPolicy::Custom(
                g as Arc<dyn agent_rs_lib::agent::permission::PermissionGate + Send + Sync>,
            )
        })
        .unwrap_or_else(|| {
            tracing::error!("AppPermissionGate not found in managed state for write_document — denying access");
            PermissionPolicy::DenyAll
        });
    let tool = WriteDocumentTool::new(sandbox, allowed_extensions, policy);
    let mut args = serde_json::json!({ "path": path, "content": content });
    if let Some(append) = append {
        args["append"] = serde_json::json!(append);
    }
    tool.call(args.to_string())
        .await
        .map_err(|e| AppError::SystemError(e.to_string()))
}

pub async fn list_directory(
    path: Option<String>,
    gate: Option<Arc<AppPermissionGate>>,
) -> Result<String, AppError> {
    let sandbox = try_get_shared_sandbox()
        .ok_or_else(|| AppError::SystemError("sandbox not initialized".into()))?;
    let policy = gate
        .map(|g| {
            PermissionPolicy::Custom(
                g as Arc<dyn agent_rs_lib::agent::permission::PermissionGate + Send + Sync>,
            )
        })
        .unwrap_or_else(|| {
            tracing::error!("AppPermissionGate not found in managed state for list_directory — denying access");
            PermissionPolicy::DenyAll
        });
    let tool = ListDirectoryTool::new(sandbox, policy);
    let args = match path {
        Some(p) => serde_json::json!({ "path": p }).to_string(),
        None => serde_json::json!({}).to_string(),
    };
    tool.call(args)
        .await
        .map_err(|e| AppError::SystemError(e.to_string()))
}

pub async fn glob_search(
    pattern: String,
    gate: Option<Arc<AppPermissionGate>>,
) -> Result<String, AppError> {
    let sandbox = try_get_shared_sandbox()
        .ok_or_else(|| AppError::SystemError("sandbox not initialized".into()))?;
    let policy = gate
        .map(|g| {
            PermissionPolicy::Custom(
                g as Arc<dyn agent_rs_lib::agent::permission::PermissionGate + Send + Sync>,
            )
        })
        .unwrap_or_else(|| {
            tracing::error!("AppPermissionGate not found in managed state for glob_search — denying access");
            PermissionPolicy::DenyAll
        });
    let tool = GlobSearchTool::new(sandbox, policy);
    let args = serde_json::json!({ "pattern": pattern }).to_string();
    tool.call(args)
        .await
        .map_err(|e| AppError::SystemError(e.to_string()))
}

pub async fn grep_search(
    allowed_extensions: HashSet<String>,
    query: String,
    path: Option<String>,
    case_sensitive: Option<bool>,
    gate: Option<Arc<AppPermissionGate>>,
) -> Result<String, AppError> {
    let sandbox = try_get_shared_sandbox()
        .ok_or_else(|| AppError::SystemError("sandbox not initialized".into()))?;
    let policy = gate
        .map(|g| {
            PermissionPolicy::Custom(
                g as Arc<dyn agent_rs_lib::agent::permission::PermissionGate + Send + Sync>,
            )
        })
        .unwrap_or_else(|| {
            tracing::error!("AppPermissionGate not found in managed state for grep_search — denying access");
            PermissionPolicy::DenyAll
        });
    let tool = GrepSearchTool::new(sandbox, allowed_extensions, policy);
    let mut args = serde_json::json!({ "query": query });
    if let Some(path) = path {
        args["path"] = serde_json::json!(path);
    }
    if let Some(case_sensitive) = case_sensitive {
        args["case_sensitive"] = serde_json::json!(case_sensitive);
    }
    tool.call(args.to_string())
        .await
        .map_err(|e| AppError::SystemError(e.to_string()))
}
