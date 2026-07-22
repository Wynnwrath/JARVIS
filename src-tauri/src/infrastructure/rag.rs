use std::path::Path;
use std::sync::Arc;

use agent_rs::agent::embeddings::{ort, EmbeddingService, FastembedModel};
use agent_rs::rag::{BuiltRag, ErasedEmbedder, RagPipeline, TurboVectorIndex};
use tokio::sync::RwLock;

use crate::domain::config::AppConfig;
use crate::domain::errors::AppError;

pub struct RagManager {
    inner: RwLock<Option<Arc<BuiltRag>>>,
    config_fingerprint: RwLock<Option<String>>,
    index_path: RwLock<Option<std::path::PathBuf>>,
    embedder: RwLock<Option<Arc<dyn ErasedEmbedder>>>,
}

impl RagManager {
    #[allow(clippy::new_without_default)]
    pub fn new() -> Self {
        Self {
            inner: RwLock::new(None),
            config_fingerprint: RwLock::new(None),
            index_path: RwLock::new(None),
            embedder: RwLock::new(None),
        }
    }

    pub async fn get_or_init(
        &self,
        config: &AppConfig,
        app_data_dir: &Path,
    ) -> Result<Arc<BuiltRag>, AppError> {
        let fingerprint = format!("{}:{}", config.embedding_model, config.rag_use_gpu);

        {
            let inner = self.inner.read().await;
            let fp = self.config_fingerprint.read().await;
            if inner.is_some() && *fp == Some(fingerprint.clone()) {
                return Ok(Arc::clone(inner.as_ref().unwrap()));
            }
        }

        let model: FastembedModel = config
            .embedding_model
            .parse()
            .map_err(|e| AppError::SystemError(format!("invalid embedding model: {}", e)))?;

        let mut eps = Vec::new();
        if config.rag_use_gpu {
            #[cfg(target_os = "windows")]
            eps.push(ort::ep::DirectML::default().build());
            #[cfg(target_os = "linux")]
            {
                eps.push(ort::ep::CUDA::default().build());
                eps.push(ort::ep::ROCm::default().build());
            }
        }
        eps.push(ort::ep::CPU::default().build());

        let svc = EmbeddingService::from_fastembed_with_providers_and_cache_dir(
            model,
            eps,
            app_data_dir.join("embedding_cache"),
        )
        .map_err(|e| AppError::SystemError(format!("failed to create embedding service: {}", e)))?;

        let embedder: Arc<dyn ErasedEmbedder> = Arc::new(svc.clone());

        let rag_data = app_data_dir.join("rag_data");
        let built = RagPipeline::builder()
            .embedder(svc)
            .store_at(&rag_data)
            .extensions(["txt", "md", "pdf"])
            .build()
            .await
            .map_err(|e| AppError::SystemError(format!("failed to build RAG pipeline: {}", e)))?;

        let arc = Arc::new(built);

        {
            let mut inner = self.inner.write().await;
            let mut fp = self.config_fingerprint.write().await;
            let mut ip = self.index_path.write().await;
            let mut emb = self.embedder.write().await;
            *inner = Some(Arc::clone(&arc));
            *fp = Some(fingerprint);
            *ip = Some(rag_data.join("rag.tvim"));
            *emb = Some(embedder);
        }

        Ok(arc)
    }

    pub async fn get(&self) -> Option<Arc<BuiltRag>> {
        self.inner.read().await.as_ref().map(Arc::clone)
    }

    pub async fn vector_index_view(&self) -> Option<TurboVectorIndex> {
        let inner = self.inner.read().await;
        let embedder = self.embedder.read().await;
        match (inner.as_ref(), embedder.as_ref()) {
            (Some(rag), Some(emb)) => Some(rag.indexer.pipeline().build(Arc::clone(emb))),
            _ => None,
        }
    }

    pub async fn clear(&self, config: &AppConfig, app_data_dir: &Path) -> Result<(), AppError> {
        {
            let mut inner = self.inner.write().await;
            let mut fp = self.config_fingerprint.write().await;
            let mut ip = self.index_path.write().await;
            let mut emb = self.embedder.write().await;
            *inner = None;
            *fp = None;
            *ip = None;
            *emb = None;
        }

        let rag_data = app_data_dir.join("rag_data");
        let _ = std::fs::remove_file(rag_data.join("rag.db"));
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
