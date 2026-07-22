use std::collections::HashMap;
use std::path::Path;

use agent_rs::rag::{Document, DocumentLoader};

use super::read_document;

pub struct JarvisPdfLoader;

#[async_trait::async_trait]
impl DocumentLoader for JarvisPdfLoader {
    async fn load(&self, path: &Path) -> anyhow::Result<Document> {
        let path_owned = path.to_path_buf();
        let text =
            tokio::task::spawn_blocking(move || read_document::extract_pdf_text(&path_owned))
                .await
                .map_err(|e| anyhow::anyhow!("spawn_blocking failed: {e}"))?
                .map_err(|e| anyhow::anyhow!("PDF extraction failed: {e}"))?;

        let source_name = path
            .file_name()
            .and_then(|n| n.to_str())
            .unwrap_or("unknown")
            .to_string();

        let mut metadata = HashMap::new();
        metadata.insert("source".to_string(), source_name);
        metadata.insert("file_type".to_string(), "pdf".to_string());

        Ok(Document {
            content: text,
            metadata,
        })
    }
}
