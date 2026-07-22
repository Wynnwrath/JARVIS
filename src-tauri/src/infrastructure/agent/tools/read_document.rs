use agent_rs::agent::permission::PermissionPolicy;
use agent_rs::domain::errors::DocumentError;
use agent_rs::security::SharedSandbox;
use rig_core::tool::Tool;
use serde_json::json;
use std::collections::HashSet;
use std::path::Path;
use std::sync::Arc;

#[derive(Debug, serde::Deserialize, serde::Serialize)]
pub struct ReadDocumentArgs {
    pub path: String,
}

#[derive(Debug, Clone)]
pub struct ReadDocumentTool {
    sandbox: Arc<SharedSandbox>,
    allowed_extensions: HashSet<String>,
    policy: PermissionPolicy,
}

impl ReadDocumentTool {
    pub fn new(
        sandbox: Arc<SharedSandbox>,
        allowed_extensions: HashSet<String>,
        policy: PermissionPolicy,
    ) -> Self {
        Self {
            sandbox,
            allowed_extensions,
            policy,
        }
    }
}

impl Tool for ReadDocumentTool {
    const NAME: &'static str = "read_document";

    type Error = DocumentError;
    type Args = ReadDocumentArgs;
    type Output = String;

    fn description(&self) -> String {
        let supported = self
            .allowed_extensions
            .iter()
            .map(|e| format!(".{e}"))
            .collect::<Vec<_>>()
            .join(", ");
        format!(
            "Read the content of a document or text file. Supports: {supported}. Paths are resolved within the configured sandbox root(s)."
        )
    }

    fn parameters(&self) -> serde_json::Value {
        json!({
            "type": "object",
            "properties": {
                "path": {
                    "type": "string",
                    "description": "The path to the file to read (relative to the sandbox root)"
                }
            },
            "required": ["path"]
        })
    }

    async fn call(&self, args: Self::Args) -> Result<Self::Output, Self::Error> {
        let description = format!("Wants to read file asset at [{}]", args.path);
        let path = self
            .sandbox
            .resolve_path_with_permission(
                &self.policy,
                Self::NAME,
                &description,
                Path::new(&args.path),
            )
            .await?;
        let extension = path.extension().and_then(|ext| ext.to_str()).unwrap_or("");

        if !self.allowed_extensions.contains(extension) {
            return Err(DocumentError::UnsupportedExtension(extension.to_string()));
        }

        match extension {
            "pdf" => {
                let path_owned = path.clone();
                tokio::task::spawn_blocking(move || extract_pdf_text(&path_owned))
                    .await
                    .map_err(|e| DocumentError::Pdf(format!("PDF task failed: {e}")))?
            }
            _ => {
                let content = tokio::fs::read_to_string(&path).await?;
                Ok(content)
            }
        }
    }
}

pub(crate) fn extract_pdf_text(path: &Path) -> Result<String, DocumentError> {
    let doc = pdf_oxide::PdfDocument::open(path)
        .map_err(|e| DocumentError::Pdf(format!("failed to open PDF: {e}")))?;
    let page_count = doc
        .page_count()
        .map_err(|e| DocumentError::Pdf(format!("failed to get page count: {e}")))?;
    let mut text = String::new();
    for i in 0..page_count {
        match doc.extract_text_auto(i) {
            Ok(page_text) => {
                if !text.is_empty() {
                    text.push('\n');
                }
                text.push_str(&page_text);
            }
            Err(e) => {
                return Err(DocumentError::Pdf(format!(
                    "page {} extraction failed: {e}",
                    i + 1
                )));
            }
        }
    }
    Ok(text)
}
