use std::{path::Path, sync::Arc};

use anyhow::{Result, anyhow};
use async_trait::async_trait;

use super::document_manager::{DocumentManager, FileRepository, normalize_extension};

#[async_trait]
pub trait DocumentExtractor: Send + Sync {
    async fn extract(&self, file_path: &Path, manager: &DocumentManager) -> Result<String>;
}

#[derive(Clone)]
pub struct Utf8DocumentExtractor {
    file_repo: Arc<dyn FileRepository>,
}

impl Utf8DocumentExtractor {
    pub fn new(file_repo: Arc<dyn FileRepository>) -> Self {
        Self { file_repo }
    }
}

#[async_trait]
impl DocumentExtractor for Utf8DocumentExtractor {
    async fn extract(&self, file_path: &Path, manager: &DocumentManager) -> Result<String> {
        let bytes = self.file_repo.read(file_path).await?;
        if bytes.is_empty() {
            return Err(anyhow!("file content is empty"));
        }

        let extension = file_path
            .extension()
            .and_then(|ext| ext.to_str())
            .map(normalize_extension)
            .unwrap_or_default();

        if !manager.is_supported_file(file_path.file_name().and_then(|n| n.to_str()).unwrap_or(""))
        {
            return Err(anyhow!("unsupported file type: {}", extension));
        }

        let text = String::from_utf8(bytes).map_err(|_| anyhow!("file is not valid UTF-8"))?;
        if text.trim().is_empty() {
            return Err(anyhow!("file contains only whitespace"));
        }

        Ok(text)
    }
}
