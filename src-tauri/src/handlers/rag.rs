use std::collections::HashMap;
use std::path::Path;
use std::sync::Arc;
use std::time::UNIX_EPOCH;

use agent_rs::rag::{BuiltRag, RagChunkRow};
use rig_core::vector_store::request::VectorSearchRequest;
use rig_core::vector_store::VectorStoreIndex;
use serde::Serialize;
use uuid::Uuid;

use crate::domain::config::AppConfig;
use crate::domain::errors::AppError;

#[derive(Debug, Clone, Serialize)]
pub struct RagTelemetryResponse {
    pub total_notes: u64,
    pub indexed_notes: u64,
    pub total_chunks: i64,
    pub db_size: String,
}

#[derive(Debug, Clone, Serialize)]
pub struct DiscoveredFolderResponse {
    pub id: String,
    pub name: String,
    pub count: u64,
    pub excluded: bool,
}

#[derive(Debug, Clone, Serialize)]
pub struct SearchResultResponse {
    pub note: String,
    pub score: f32,
    pub content: String,
}

#[derive(Debug, Clone, Serialize)]
pub struct IndexingProgressPayload {
    pub level: String,
    pub message: String,
    pub progress: u8,
}

#[derive(Debug, Clone, Serialize)]
pub struct RagDirEntry {
    pub name: String,
    pub is_dir: bool,
    pub path: String,
    pub size: Option<u64>,
}

fn assert_within_rag_dirs(path: &Path, config: &AppConfig) -> Result<std::path::PathBuf, AppError> {
    let canonical = path
        .canonicalize()
        .map_err(|e| AppError::SystemError(format!("failed to resolve path: {}", e)))?;
    for dir in &config.rag_dirs {
        if let Ok(root) = Path::new(dir).canonicalize() {
            if canonical.starts_with(&root) {
                return Ok(canonical);
            }
        }
    }
    Err(AppError::SystemError(
        "path is not within a configured RAG directory".into(),
    ))
}

fn count_indexable_files(dir: &Path) -> u64 {
    let mut count = 0;
    if let Ok(entries) = std::fs::read_dir(dir) {
        for entry in entries.flatten() {
            let path = entry.path();
            if path.is_dir() {
                count += count_indexable_files(&path);
            } else if let Some(ext) = path.extension().and_then(|e| e.to_str()) {
                if matches!(ext, "txt" | "md" | "pdf") {
                    count += 1;
                }
            }
        }
    }
    count
}

const MAX_INDEX_FILE_BYTES: u64 = 200 * 1024 * 1024;

fn collect_indexable_files(dir: &Path) -> Vec<std::path::PathBuf> {
    let mut out = Vec::new();
    if let Ok(entries) = std::fs::read_dir(dir) {
        for entry in entries.flatten() {
            let path = entry.path();
            if path.is_dir() {
                out.extend(collect_indexable_files(&path));
            } else if let Some(ext) = path.extension().and_then(|e| e.to_str()) {
                if matches!(ext, "txt" | "md" | "pdf") {
                    out.push(path);
                }
            }
        }
    }
    out
}

fn human_readable_size(bytes: u64) -> String {
    const UNITS: &[&str] = &["B", "KB", "MB", "GB"];
    let mut size = bytes as f64;
    let mut unit_idx = 0;
    while size >= 1024.0 && unit_idx < UNITS.len() - 1 {
        size /= 1024.0;
        unit_idx += 1;
    }
    if unit_idx == 0 {
        format!("{} {}", bytes, UNITS[unit_idx])
    } else {
        format!("{:.1} {}", size, UNITS[unit_idx])
    }
}

pub async fn get_telemetry(
    config: &AppConfig,
    rag: Option<&Arc<BuiltRag>>,
    app_data_dir: &Path,
) -> Result<RagTelemetryResponse, AppError> {
    let total_chunks = match rag {
        Some(r) => r.indexer.chunk_count().await.unwrap_or(0),
        None => 0,
    };

    let indexed_notes = rag.map(|r| r.indexer.list().len() as u64).unwrap_or(0);

    let total_notes = {
        let mut count = 0;
        if !config.sandbox_dir.is_empty() && config.sandbox_dir != "." {
            let vault = Path::new(&config.sandbox_dir);
            if vault.exists() {
                count += count_indexable_files(vault);
            }
        }
        for dir in &config.rag_dirs {
            let path = Path::new(dir);
            if path.exists() {
                count += count_indexable_files(path);
            }
        }
        count
    };

    let rag_data = app_data_dir.join("rag_data");
    let db_bytes = std::fs::metadata(rag_data.join("rag.db"))
        .map(|m| m.len())
        .unwrap_or(0);
    let tvim_bytes = std::fs::metadata(rag_data.join("rag.tvim"))
        .map(|m| m.len())
        .unwrap_or(0);
    let db_size = human_readable_size(db_bytes + tvim_bytes);

    Ok(RagTelemetryResponse {
        total_notes,
        indexed_notes,
        total_chunks,
        db_size,
    })
}

pub async fn get_directories(
    vault_path: &str,
    config: &AppConfig,
) -> Result<Vec<DiscoveredFolderResponse>, AppError> {
    let vault = Path::new(vault_path);
    if !vault.exists() {
        return Ok(Vec::new());
    }

    let mut folders = Vec::new();
    let mut entries = std::fs::read_dir(vault)
        .map_err(|e| AppError::SystemError(format!("failed to read vault directory: {}", e)))?;

    while let Some(entry) = entries.next().transpose()? {
        let path = entry.path();
        if !path.is_dir() {
            continue;
        }
        let name = entry.file_name().to_string_lossy().into_owned();
        let count = count_indexable_files(&path);
        let excluded = config.rag_exclusions.contains(&name);
        folders.push(DiscoveredFolderResponse {
            id: Uuid::new_v4().to_string(),
            name,
            count,
            excluded,
        });
    }

    Ok(folders)
}

pub async fn toggle_exclusion(
    dir_name: &str,
    config: &mut AppConfig,
    rag: &BuiltRag,
) -> Result<bool, AppError> {
    let vault_path = config.sandbox_dir.clone();
    if vault_path.is_empty() || vault_path == "." {
        return Err(AppError::SystemError(
            "sandbox_dir is not configured".into(),
        ));
    }

    let dir_path = Path::new(&vault_path).join(dir_name);
    let was_excluded = config.rag_exclusions.iter().any(|e| e == dir_name);

    if was_excluded {
        config.rag_exclusions.retain(|e| e != dir_name);
        for file in collect_indexable_files(&dir_path) {
            let size = std::fs::metadata(&file).map(|m| m.len()).unwrap_or(0);
            if size > MAX_INDEX_FILE_BYTES {
                tracing::warn!("skipping oversized file (>50 MB): {}", file.display());
                continue;
            }
            if let Err(e) = rag.indexer.add(&file).await {
                tracing::warn!("failed to index {}: {}", file.display(), e);
            }
        }
    } else {
        config.rag_exclusions.push(dir_name.to_string());
        rag.indexer.remove(&dir_path).await.map_err(|e| {
            AppError::SystemError(format!("failed to remove directory from index: {}", e))
        })?;
    }

    Ok(!was_excluded)
}

fn collect_files_recursive(
    root: &Path,
    dir: &Path,
    out: &mut Vec<RagDirEntry>,
) -> Result<(), AppError> {
    let entries = std::fs::read_dir(dir)
        .map_err(|e| AppError::SystemError(format!("failed to read directory: {}", e)))?;
    for entry in entries {
        let entry =
            entry.map_err(|e| AppError::SystemError(format!("failed to read entry: {}", e)))?;
        let path = entry.path();
        if path.is_dir() {
            collect_files_recursive(root, &path, out)?;
        } else if let Some(ext) = path.extension().and_then(|e| e.to_str()) {
            if matches!(ext, "txt" | "md" | "pdf") {
                let relative = path.strip_prefix(root).unwrap_or(&path);
                let size = entry.metadata().ok().map(|m| m.len());
                out.push(RagDirEntry {
                    name: relative.to_string_lossy().replace('\\', "/"),
                    is_dir: false,
                    path: path.to_string_lossy().into_owned(),
                    size,
                });
            }
        }
    }
    Ok(())
}

pub async fn list_rag_directory(
    path: &str,
    config: &AppConfig,
) -> Result<Vec<RagDirEntry>, AppError> {
    let canonical = assert_within_rag_dirs(Path::new(path), config)?;
    let mut out = Vec::new();
    collect_files_recursive(&canonical, &canonical, &mut out)?;
    out.sort_by_key(|a| a.name.to_lowercase());
    Ok(out)
}

pub async fn read_rag_document(path: &str, config: &AppConfig) -> Result<String, AppError> {
    let canonical = assert_within_rag_dirs(Path::new(path), config)?;
    if canonical.is_dir() {
        return Err(AppError::SystemError("path is a directory".into()));
    }
    std::fs::read_to_string(&canonical)
        .map_err(|e| AppError::SystemError(format!("failed to read document: {}", e)))
}

pub async fn remove_rag_dir(
    dir_path: &str,
    config: &mut AppConfig,
    rag: Option<&BuiltRag>,
) -> Result<(), AppError> {
    if !config.rag_dirs.iter().any(|d| d == dir_path) {
        return Err(AppError::SystemError(
            "directory is not a configured RAG directory".into(),
        ));
    }

    if let Some(r) = rag {
        if let Err(e) = r.indexer.remove(Path::new(dir_path)).await {
            tracing::warn!("de-index on rag dir removal reported: {}", e);
        }
    }

    config.rag_dirs.retain(|d| d != dir_path);
    Ok(())
}

pub async fn query_sandbox(
    query: &str,
    rag: &BuiltRag,
) -> Result<Vec<SearchResultResponse>, AppError> {
    let req = VectorSearchRequest::builder()
        .query(query)
        .samples(5)
        .build();

    let hits = rag
        .vector_index
        .top_n_ids(req)
        .await
        .map_err(|e| AppError::SystemError(format!("vector search failed: {}", e)))?;

    if hits.is_empty() {
        return Ok(Vec::new());
    }

    let ids: Vec<i64> = hits
        .iter()
        .filter_map(|(_, id_str)| id_str.parse::<i64>().ok())
        .collect();

    let rows = rag
        .indexer
        .pipeline()
        .store()
        .get_chunks_by_ids(&ids)
        .await
        .map_err(|e| AppError::SystemError(format!("failed to fetch chunk rows: {}", e)))?;

    let row_map: std::collections::HashMap<i64, &RagChunkRow> =
        rows.iter().map(|r| (r.id, r)).collect();

    let results = hits
        .iter()
        .filter_map(|(score, id_str)| {
            let id = id_str.parse::<i64>().ok()?;
            let row = row_map.get(&id)?;
            Some(SearchResultResponse {
                note: row.source.clone(),
                score: *score as f32,
                content: row.content.clone(),
            })
        })
        .collect();

    Ok(results)
}

pub async fn start_indexing(
    vault_path: &str,
    config: &AppConfig,
    rag: &Arc<BuiltRag>,
    app_data_dir: &Path,
    force: bool,
    emit: impl Fn(IndexingProgressPayload),
) -> Result<(), AppError> {
    emit(IndexingProgressPayload {
        level: "INIT".into(),
        message: format!("[START] Commencing directory scan at \"{}\"", vault_path),
        progress: 0,
    });

    let vault = Path::new(vault_path);

    let mut dirs = Vec::new();
    if vault_path.is_empty() || !vault.exists() {
        if !vault_path.is_empty() {
            emit(IndexingProgressPayload {
                level: "INDEX".into(),
                message: format!("[INDEX] Vault path not found, skipping: {}", vault_path),
                progress: 0,
            });
        }
    } else if let Ok(entries) = std::fs::read_dir(vault) {
        for entry in entries.flatten() {
            let path = entry.path();
            if path.is_dir() {
                let name = entry.file_name().to_string_lossy().into_owned();
                if !config.rag_exclusions.contains(&name) {
                    dirs.push(path);
                }
            }
        }
    }

    for dir in &config.rag_dirs {
        let path = Path::new(dir);
        if path.is_dir() {
            dirs.push(path.to_path_buf());
        } else {
            emit(IndexingProgressPayload {
                level: "INDEX".into(),
                message: format!("[INDEX] Indexed dir missing, skipping: {}", dir),
                progress: 0,
            });
        }
    }

    let mut file_groups: Vec<Vec<std::path::PathBuf>> = Vec::new();
    let mut total_files = 0usize;
    for dir_path in &dirs {
        let name = dir_path
            .file_name()
            .and_then(|n| n.to_str())
            .unwrap_or("unknown");
        let mut files = Vec::new();
        for file in collect_indexable_files(dir_path) {
            let size = std::fs::metadata(&file).map(|m| m.len()).unwrap_or(0);
            if size > MAX_INDEX_FILE_BYTES {
                emit(IndexingProgressPayload {
                    level: "INDEX".into(),
                    message: format!(
                        "[INDEX] Skipping oversized file (>50 MB): {}",
                        file.display()
                    ),
                    progress: 0,
                });
            } else {
                files.push(file);
            }
        }
        emit(IndexingProgressPayload {
            level: "INDEX".into(),
            message: format!("[INDEX] Indexing {}: {} files", name, files.len()),
            progress: 0,
        });
        total_files += files.len();
        file_groups.push(files);
    }

    if total_files == 0 {
        emit(IndexingProgressPayload {
            level: "ERROR".into(),
            message: "no indexable files found".into(),
            progress: 0,
        });
        return Err(AppError::SystemError("no indexable files found".into()));
    }

    let mtimes_path = app_data_dir.join("rag_data").join("mtimes.json");
    let mut mtimes: HashMap<String, u64> = if mtimes_path.exists() {
        std::fs::read_to_string(&mtimes_path)
            .ok()
            .and_then(|s| serde_json::from_str(&s).ok())
            .unwrap_or_default()
    } else {
        HashMap::new()
    };

    let mut done = 0usize;
    let mut last_pct = 0u8;
    let mut newly_indexed = 0u64;
    let mut reindexed = 0u64;
    let mut skipped = 0u64;
    let mut failed = 0u64;

    for files in &file_groups {
        for file in files {
            let current_mtime = std::fs::metadata(file)
                .ok()
                .and_then(|m| m.modified().ok())
                .and_then(|t| t.duration_since(UNIX_EPOCH).ok())
                .map(|d| d.as_secs());

            let canonical = file.to_string_lossy().to_string();

            let should_skip = if force {
                false
            } else if let Some(&stored_mtime) = mtimes.get(&canonical) {
                current_mtime
                    .map(|cur| cur <= stored_mtime)
                    .unwrap_or(false)
            } else {
                false
            };

            if should_skip {
                skipped += 1;
                done += 1;
                let pct = ((done as f32 / total_files as f32) * 90.0) as u8;
                if pct != last_pct {
                    last_pct = pct.min(90);
                    emit(IndexingProgressPayload {
                        level: "INDEX".into(),
                        message: String::new(),
                        progress: last_pct,
                    });
                }
                continue;
            }

            let is_new = !mtimes.contains_key(&canonical);
            let result = if force || !is_new {
                rag.indexer.reindex(file).await
            } else {
                rag.indexer.add(file).await
            };

            match result {
                Ok(_) => {
                    if force || !is_new {
                        reindexed += 1;
                    } else {
                        newly_indexed += 1;
                    }
                    if let Some(mtime_secs) = current_mtime {
                        mtimes.insert(canonical, mtime_secs);
                    }
                }
                Err(e) => {
                    emit(IndexingProgressPayload {
                        level: "INDEX".into(),
                        message: format!(
                            "[INDEX] Failed to index {}: {} — skipping",
                            file.display(),
                            e
                        ),
                        progress: last_pct,
                    });
                    failed += 1;
                }
            }
            done += 1;
            let pct = ((done as f32 / total_files as f32) * 90.0) as u8;
            if pct != last_pct {
                last_pct = pct.min(90);
                emit(IndexingProgressPayload {
                    level: "INDEX".into(),
                    message: String::new(),
                    progress: last_pct,
                });
            }
        }
    }

    emit(IndexingProgressPayload {
        level: "OPTIMIZE".into(),
        message: "[OPTIMIZE] Persisting vector index to disk...".into(),
        progress: 95,
    });

    let index_path = app_data_dir.join("rag_data").join("rag.tvim");
    if let Err(e) = rag.indexer.pipeline().save(&index_path).await {
        emit(IndexingProgressPayload {
            level: "ERROR".into(),
            message: format!("failed to save vector index: {}", e),
            progress: 95,
        });
        return Err(AppError::SystemError(format!(
            "failed to save vector index: {}",
            e
        )));
    }

    if let Some(dir) = mtimes_path.parent() {
        let _ = std::fs::create_dir_all(dir);
    }
    if let Ok(json) = serde_json::to_string(&mtimes) {
        let _ = std::fs::write(&mtimes_path, json);
    }

    emit(IndexingProgressPayload {
        level: "SUCCESS".into(),
        message: format!(
            "[SUCCESS] Ingestion completed. New: {}, Reindexed: {}, Skipped: {}, Failed: {}",
            newly_indexed, reindexed, skipped, failed
        ),
        progress: 100,
    });

    Ok(())
}
