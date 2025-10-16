use std::{
    collections::{HashMap, HashSet},
    sync::Arc,
};

use anyhow::Error;
use serde_json::json;

use crate::storage::{DocProcessingStatus, DocStatus, DocStatusStorage, KvStorage, StorageResult};

#[derive(Debug, Clone)]
pub struct PendingDocument {
    pub id: String,
    pub content: String,
    pub summary: String,
    pub length: i64,
    pub file_path: String,
    pub track_id: String,
    pub created_at: String,
}

pub struct DocStatusService {
    doc_status: Arc<dyn DocStatusStorage>,
    docs_storage: Arc<dyn KvStorage>,
}

impl DocStatusService {
    pub fn new(doc_status: Arc<dyn DocStatusStorage>, docs_storage: Arc<dyn KvStorage>) -> Self {
        Self {
            doc_status,
            docs_storage,
        }
    }

    pub async fn filter_new_ids(
        &self,
        doc_ids: &HashSet<String>,
    ) -> StorageResult<HashSet<String>> {
        self.doc_status.filter_keys(doc_ids).await
    }

    pub async fn enqueue_pending(&self, documents: Vec<PendingDocument>) -> StorageResult<()> {
        if documents.is_empty() {
            return Ok(());
        }

        let mut docs_payload = HashMap::new();
        let mut status_payload = HashMap::new();

        for doc in documents {
            docs_payload.insert(doc.id.clone(), json!({ "content": doc.content }));
            status_payload.insert(
                doc.id.clone(),
                DocProcessingStatus {
                    id: Some(doc.id),
                    status: DocStatus::PENDING,
                    content_summary: Some(doc.summary),
                    content_length: Some(doc.length),
                    created_at: Some(doc.created_at.clone()),
                    updated_at: Some(doc.created_at),
                    file_path: Some(doc.file_path),
                    track_id: Some(doc.track_id),
                    chunks_list: Some(vec![]),
                    metadata: None,
                    error_msg: None,
                },
            );
        }

        self.docs_storage.upsert(docs_payload).await?;
        self.doc_status.upsert(status_payload).await
    }

    pub async fn mark_processing(
        &self,
        doc_id: &str,
        status: &DocProcessingStatus,
        chunk_ids: &[String],
    ) -> StorageResult<()> {
        let now = chrono::Utc::now().to_rfc3339();
        let mut payload = HashMap::new();
        payload.insert(
            doc_id.to_string(),
            DocProcessingStatus {
                id: Some(doc_id.to_string()),
                status: DocStatus::PROCESSING,
                content_summary: status.content_summary.clone(),
                content_length: status.content_length,
                created_at: status.created_at.clone(),
                updated_at: Some(now),
                file_path: status.file_path.clone(),
                track_id: status.track_id.clone(),
                chunks_list: Some(chunk_ids.to_vec()),
                metadata: status.metadata.clone(),
                error_msg: None,
            },
        );

        self.doc_status.upsert(payload).await
    }

    pub async fn mark_processed(
        &self,
        doc_id: &str,
        status: &DocProcessingStatus,
        chunk_ids: &[String],
    ) -> StorageResult<()> {
        let now = chrono::Utc::now().to_rfc3339();
        let mut payload = HashMap::new();
        payload.insert(
            doc_id.to_string(),
            DocProcessingStatus {
                id: Some(doc_id.to_string()),
                status: DocStatus::PROCESSED,
                content_summary: status.content_summary.clone(),
                content_length: status.content_length,
                created_at: status.created_at.clone(),
                updated_at: Some(now),
                file_path: status.file_path.clone(),
                track_id: status.track_id.clone(),
                chunks_list: Some(chunk_ids.to_vec()),
                metadata: status.metadata.clone(),
                error_msg: None,
            },
        );

        self.doc_status.upsert(payload).await
    }

    pub async fn mark_failed(
        &self,
        doc_id: &str,
        status: &DocProcessingStatus,
        err: &Error,
    ) -> StorageResult<()> {
        let now = chrono::Utc::now().to_rfc3339();
        let mut payload = HashMap::new();
        payload.insert(
            doc_id.to_string(),
            DocProcessingStatus {
                id: Some(doc_id.to_string()),
                status: DocStatus::FAILED,
                content_summary: status.content_summary.clone(),
                content_length: status.content_length,
                created_at: status.created_at.clone(),
                updated_at: Some(now),
                file_path: status.file_path.clone(),
                track_id: status.track_id.clone(),
                chunks_list: Some(vec![]),
                metadata: status.metadata.clone(),
                error_msg: Some(err.to_string()),
            },
        );

        self.doc_status.upsert(payload).await
    }
}
