use std::{collections::HashMap, path::Path, sync::Arc};

use anyhow::Result;
use serde_json::json;

use crate::storage::{DocProcessingStatus, DocStatus, DocStatusStorage};

use super::utils::compute_mdhash_id;

pub struct ErrorReporter {
    storage: Arc<dyn DocStatusStorage>,
}

impl ErrorReporter {
    pub fn new(storage: Arc<dyn DocStatusStorage>) -> Self {
        Self { storage }
    }

    pub async fn record(
        &self,
        file_path: &Path,
        track_id: &str,
        error_type: &str,
        err: &anyhow::Error,
    ) -> Result<()> {
        let filename = file_path
            .file_name()
            .and_then(|n| n.to_str())
            .unwrap_or("unknown")
            .to_string();
        let current_time = chrono::Utc::now().to_rfc3339();

        let error_doc = DocProcessingStatus {
            id: None,
            status: DocStatus::FAILED,
            content_summary: Some(format!("{error_type} failed for {filename}")),
            content_length: Some(0),
            created_at: Some(current_time.clone()),
            updated_at: Some(current_time),
            file_path: Some(filename.clone()),
            track_id: Some(track_id.to_string()),
            chunks_list: Some(vec![]),
            metadata: Some(json!({
                "error_type": error_type,
                "error_message": err.to_string(),
            })),
            error_msg: Some(err.to_string()),
        };

        let doc_id = compute_mdhash_id(&format!("error-{track_id}-{filename}"), "error-");
        let mut payload = HashMap::new();
        payload.insert(doc_id, error_doc);

        self.storage.upsert(payload).await?;
        Ok(())
    }
}
