use std::collections::{HashMap, HashSet};

use anyhow::Result;
use async_trait::async_trait;
use serde::{Deserialize, Serialize};

pub mod io;
pub mod json_doc_status;
pub mod json_kv;

pub use io::*;
pub use json_doc_status::{JsonDocStatusConfig, JsonDocStatusStorage};
pub use json_kv::{JsonKvStorage, JsonKvStorageConfig};

pub type StorageResult<T> = Result<T>;

#[async_trait]
pub trait KvStorage: Send + Sync {
    async fn initialize(&self) -> StorageResult<()>;
    async fn finalize(&self) -> StorageResult<()>;

    async fn upsert(&self, records: HashMap<String, serde_json::Value>) -> StorageResult<()>;

    async fn delete(&self, ids: &[String]) -> StorageResult<()>;
    async fn drop_all(&self) -> StorageResult<()>;

    async fn get_all(&self) -> StorageResult<HashMap<String, serde_json::Value>>;
    async fn get_by_id(&self, id: &str) -> StorageResult<Option<serde_json::Value>>;
    async fn get_by_ids(&self, ids: &[String]) -> StorageResult<Vec<Option<serde_json::Value>>>;

    async fn filter_keys(&self, keys: &HashSet<String>) -> StorageResult<HashSet<String>>;

    /// Flush dirty state to disk if needed (Python's `index_done_callback`).
    async fn sync_if_dirty(&self) -> StorageResult<()>;
}

#[derive(Debug, Default, Clone, Serialize, Deserialize, Eq, PartialEq, Hash)]
pub enum DocStatus {
    #[default]
    PENDING,
    PROCESSING,
    PROCESSED,
    FAILED,
    ALL,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct DocProcessingStatus {
    #[serde(default)]
    pub id: Option<String>,
    pub status: DocStatus,
    pub content_summary: Option<String>,
    pub content_length: Option<i64>,
    pub created_at: Option<String>,
    pub updated_at: Option<String>,
    pub file_path: Option<String>,
    pub track_id: Option<String>,
    pub chunks_list: Option<Vec<String>>,
    pub metadata: Option<serde_json::Value>,
    pub error_msg: Option<String>,
}

#[async_trait]
pub trait DocStatusStorage: Send + Sync {
    async fn initialize(&self) -> StorageResult<()>;
    async fn finalize(&self) -> StorageResult<()>;

    async fn upsert(&self, records: HashMap<String, DocProcessingStatus>) -> StorageResult<()>;

    async fn delete(&self, ids: &[String]) -> StorageResult<()>;
    async fn drop_all(&self) -> StorageResult<()>;

    async fn get_by_id(&self, id: &str) -> StorageResult<Option<DocProcessingStatus>>;
    async fn get_by_ids(&self, ids: &[String]) -> StorageResult<Vec<Option<DocProcessingStatus>>>;

    async fn get_doc_by_file_path(
        &self,
        file_path: &str,
    ) -> StorageResult<Option<DocProcessingStatus>>;

    async fn filter_keys(&self, keys: &HashSet<String>) -> StorageResult<HashSet<String>>;

    async fn status_counts(&self) -> StorageResult<HashMap<DocStatus, usize>>;
    async fn status_counts_with_total(&self) -> StorageResult<HashMap<DocStatus, usize>>;

    async fn docs_by_status(
        &self,
        status: &DocStatus,
    ) -> StorageResult<HashMap<String, DocProcessingStatus>>;

    async fn docs_by_track_id(
        &self,
        track_id: &str,
    ) -> StorageResult<HashMap<String, DocProcessingStatus>>;

    async fn docs_paginated(
        &self,
        status_filter: Option<&DocStatus>,
        page: usize,
        page_size: usize,
        sort_field: &str,
        sort_direction: &str,
    ) -> StorageResult<(Vec<(String, DocProcessingStatus)>, usize)>;

    async fn sync_if_dirty(&self) -> StorageResult<()>;
}
