use std::path::Path;
use std::sync::Arc;

use agent_rs::agent::embeddings::{ort, EmbeddingService, FastembedModel};
use agent_rs::rag::{BuiltRag, RagPipeline};
use tokio::sync::RwLock;

use crate::domain::config::AppConfig;
use crate::domain::errors::AppError;
use crate::infrastructure::agent::tools::JarvisPdfLoader;

pub struct RagManager {
    inner: RwLock<Option<Arc<BuiltRag>>>,
    config_fingerprint: RwLock<Option<String>>,
    index_path: RwLock<Option<std::path::PathBuf>>,
}

impl RagManager {
    #[allow(clippy::new_without_default)]
    pub fn new() -> Self {
        Self {
            inner: RwLock::new(None),
            config_fingerprint: RwLock::new(None),
            index_path: RwLock::new(None),
        }
    }

            pub async fn get_or_init(
        &self,
        config: &AppConfig,
        app_data_dir: &Path,
    ) -> Result<Arc<BuiltRag>, AppError> {
        let fingerprint = format!("{}:{}", config.embedding_model, config.rag_use_gpu);
        let cache_dir = app_data_dir.join("embedding_cache");

        tracing::info!(
            model = %config.embedding_model,
            use_gpu = config.rag_use_gpu,
            cache_dir = %cache_dir.display(),
            "RAG get_or_init called"
        );

        {
            let inner = self.inner.read().await;
            let fp = self.config_fingerprint.read().await;
            if inner.is_some() && *fp == Some(fingerprint.clone()) {
                tracing::info!("RAG cache hit — returning existing instance");
                return Ok(Arc::clone(inner.as_ref().unwrap()));
            }
        }

        let model: FastembedModel = config
            .embedding_model
            .parse()
            .map_err(|e| AppError::SystemError(format!("invalid embedding model: {}", e)))?;

        tracing::info!("parsed embedding model variant");

        let mut eps = Vec::new();
        if config.rag_use_gpu {
            tracing::info!(os = std::env::consts::OS, "building GPU execution providers");
            #[cfg(target_os = "windows")]
            eps.push(ort::ep::DirectML::default().build());
            #[cfg(target_os = "linux")]
            {
                eps.push(ort::ep::CUDA::default().build());
                eps.push(ort::ep::ROCm::default().build());
            }
        }
        eps.push(ort::ep::CPU::default().build());

        tracing::info!("loading embedding model (may download from HuggingFace on first run)");

        let svc = tokio::task::spawn_blocking(move || {
            EmbeddingService::from_fastembed_with_providers_and_cache_dir(model, eps, cache_dir)
        })
        .await
        .map_err(|e| {
            tracing::error!(error = %e, "embedding service init task panicked");
            AppError::SystemError(format!("embedding service init task panicked: {}", e))
        })?
        .map_err(|e| {
            tracing::error!(error = %e, "failed to create embedding service");
            AppError::SystemError(format!("failed to create embedding service: {}", e))
        })?;

        tracing::info!("embedding service initialized");

        let rag_data = app_data_dir.join("rag_data");
        std::fs::create_dir_all(&rag_data)
            .map_err(|e| {
                tracing::error!(error = %e, "failed to create rag data dir");
                AppError::SystemError(format!("failed to create rag data dir: {}", e))
            })?;

        tracing::info!("building RAG pipeline");

        let built = RagPipeline::builder()
            .embedder(svc)
            .store_at(&rag_data)
            .extensions(["txt", "md", "pdf"])
            .loader("pdf", Arc::new(JarvisPdfLoader))
            .build()
            .await
            .map_err(|e| {
                tracing::error!(error = %e, "failed to build RAG pipeline");
                AppError::SystemError(format!("failed to build RAG pipeline: {}", e))
            })?;

        tracing::info!("RAG pipeline built successfully");

        let arc = Arc::new(built);

        {
            let mut inner = self.inner.write().await;
            let mut fp = self.config_fingerprint.write().await;
            let mut ip = self.index_path.write().await;
            *inner = Some(Arc::clone(&arc));
            *fp = Some(fingerprint);
            *ip = Some(rag_data.join("rag.tvim"));
        }

        tracing::info!("RAG pipeline initialized and cached");

        Ok(arc)
    }

    pub async fn get(&self) -> Option<Arc<BuiltRag>> {
        self.inner.read().await.as_ref().map(Arc::clone)
    }

    pub async fn clear(&self, config: &AppConfig, app_data_dir: &Path) -> Result<(), AppError> {
        {
            let mut inner = self.inner.write().await;
            let mut fp = self.config_fingerprint.write().await;
            let mut ip = self.index_path.write().await;
            *inner = None;
            *fp = None;
            *ip = None;
        }

        let rag_data = app_data_dir.join("rag_data");
        let _ = std::fs::remove_file(rag_data.join("rag.db"));
        let _ = std::fs::remove_file(rag_data.join("mtimes.json"));
        if let Ok(entries) = std::fs::read_dir(&rag_data) {
            for entry in entries.flatten() {
                let path = entry.path();
                if path.extension().map(|e| e == "tvim").unwrap_or(false) {
                    let _ = std::fs::remove_file(&path);
                }
            }
        }

        self.get_or_init(config, app_data_dir).await?;
        Ok(())
    }
}
