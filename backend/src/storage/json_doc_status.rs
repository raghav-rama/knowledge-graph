use std::{
    collections::{HashMap, HashSet},
    path::PathBuf,
    sync::{
        Arc,
        atomic::{AtomicBool, Ordering as AtomicOrdering},
    },
};

use anyhow::{Context, Result};
use async_trait::async_trait;
use serde::{Deserialize, Serialize};
use serde_json::{Map, Value};
use tokio::sync::RwLock;

use super::io::{ensure_parent_dir, load_or_default, write_json_file};
use super::{DocProcessingStatus, DocStatus, DocStatusStorage};

#[derive(Clone, Debug)]
pub struct JsonDocStatusConfig {
    pub working_dir: PathBuf,
    pub namespace: String,
    pub workspace: Option<String>,
}

pub struct JsonDocStatusStorage {
    final_namespace: String,
    file_path: PathBuf,
    data: Arc<RwLock<HashMap<String, DocRecord>>>,
    dirty: AtomicBool,
}

#[derive(Debug, Clone, Serialize, Deserialize, Default)]
struct DocRecord {
    pub status: DocStatus,

    #[serde(default)]
    pub content_summary: Option<String>,
    #[serde(default)]
    pub content_length: Option<i64>,
    #[serde(default)]
    pub created_at: Option<String>,
    #[serde(default)]
    pub updated_at: Option<String>,
    #[serde(default)]
    pub file_path: Option<String>,
    #[serde(default)]
    pub track_id: Option<String>,

    #[serde(default)]
    pub chunks_list: Vec<String>,

    #[serde(default = "empty_object")]
    pub metadata: Value,

    #[serde(default)]
    pub error_msg: Option<String>,
}

fn empty_object() -> Value {
    Value::Object(Map::new())
}

impl DocRecord {
    fn normalize(mut self) -> Self {
        if self.file_path.is_none() {
            self.file_path = Some("no-file-path".to_string());
        }
        if matches!(self.metadata, Value::Null) {
            self.metadata = empty_object();
        }
        self
    }

    fn to_status(&self, id: &str) -> DocProcessingStatus {
        DocProcessingStatus {
            id: Some(id.to_string()),
            status: self.status.clone(),
            content_summary: self.content_summary.clone(),
            content_length: self.content_length,
            created_at: self.created_at.clone(),
            updated_at: self.updated_at.clone(),
            file_path: Some(
                self.file_path
                    .clone()
                    .unwrap_or_else(|| "no-file-path".to_string()),
            ),
            track_id: self.track_id.clone(),
            chunks_list: Some(self.chunks_list.clone()),
            metadata: Some(if self.metadata.is_null() {
                empty_object()
            } else {
                self.metadata.clone()
            }),
            error_msg: self.error_msg.clone(),
        }
    }
}

impl JsonDocStatusStorage {
    pub fn new(config: JsonDocStatusConfig) -> Self {
        let JsonDocStatusConfig {
            working_dir,
            namespace,
            workspace,
        } = config;

        let (workspace_prefix, workspace_dir) = match workspace.as_deref() {
            Some(ws) if !ws.is_empty() => (ws.to_string(), working_dir.join(ws)),
            _ => ("_".to_string(), working_dir.clone()),
        };

        let final_namespace = format!("{}_{}", workspace_prefix, namespace);
        let file_path = workspace_dir.join(format!("doc_status_{}.json", namespace));

        Self {
            final_namespace,
            file_path,
            data: Arc::new(RwLock::new(HashMap::new())),
            dirty: AtomicBool::new(false),
        }
    }

    fn mark_dirty(&self) {
        self.dirty.store(true, AtomicOrdering::SeqCst);
    }

    fn build_sort_key(record: &DocRecord, id: &str, field: &str) -> String {
        match field {
            "created_at" => record.created_at.clone().unwrap_or_default(),
            "updated_at" => record.updated_at.clone().unwrap_or_default(),
            "file_path" => record
                .file_path
                .clone()
                .unwrap_or_else(|| "no-file-path".to_string())
                .to_lowercase(),
            "id" => id.to_string(),
            _ => record.updated_at.clone().unwrap_or_default(),
        }
    }
}

#[async_trait]
impl DocStatusStorage for JsonDocStatusStorage {
    async fn initialize(&self) -> Result<()> {
        ensure_parent_dir(&self.file_path).await?;
        let mut data: HashMap<String, DocRecord> = load_or_default(&self.file_path).await?;
        data = data.into_iter().map(|(k, v)| (k, v.normalize())).collect();
        *self.data.write().await = data;
        self.dirty.store(false, AtomicOrdering::SeqCst);
        Ok(())
    }

    async fn finalize(&self) -> Result<()> {
        self.sync_if_dirty().await
    }

    async fn upsert(&self, records: HashMap<String, DocProcessingStatus>) -> Result<()> {
        if records.is_empty() {
            return Ok(());
        }

        let mut guard = self.data.write().await;

        for (id, status) in records {
            let record = DocRecord {
                status: status.status,
                content_summary: status.content_summary,
                content_length: status.content_length,
                created_at: status.created_at,
                updated_at: status.updated_at,
                file_path: status.file_path,
                track_id: status.track_id,
                chunks_list: status.chunks_list.unwrap_or_default(),
                metadata: status.metadata.unwrap_or_else(empty_object),
                error_msg: status.error_msg,
            }
            .normalize();

            guard.insert(id, record);
        }

        drop(guard);
        self.mark_dirty();
        self.sync_if_dirty().await
    }

    async fn delete(&self, ids: &[String]) -> Result<()> {
        if ids.is_empty() {
            return Ok(());
        }

        let mut guard = self.data.write().await;
        let mut removed_any = false;

        for id in ids {
            if guard.remove(id).is_some() {
                removed_any = true;
            }
        }

        drop(guard);
        if removed_any {
            self.mark_dirty();
        }
        Ok(())
    }

    async fn drop_all(&self) -> Result<()> {
        {
            let mut guard = self.data.write().await;
            if guard.is_empty() {
                return Ok(());
            }
            guard.clear();
        }
        self.mark_dirty();
        self.sync_if_dirty().await
    }

    async fn get_by_id(&self, id: &str) -> Result<Option<DocProcessingStatus>> {
        let guard = self.data.read().await;
        Ok(guard.get(id).map(|record| record.to_status(id)))
    }

    async fn get_by_ids(&self, ids: &[String]) -> Result<Vec<Option<DocProcessingStatus>>> {
        let guard = self.data.read().await;
        Ok(ids
            .iter()
            .map(|id| guard.get(id).map(|record| record.to_status(id)))
            .collect())
    }

    async fn get_doc_by_file_path(&self, file_path: &str) -> Result<Option<DocProcessingStatus>> {
        let guard = self.data.read().await;
        Ok(guard.iter().find_map(|(id, record)| {
            record
                .file_path
                .as_deref()
                .filter(|fp| *fp == file_path)
                .map(|_| record.to_status(id))
        }))
    }

    async fn filter_keys(&self, keys: &HashSet<String>) -> Result<HashSet<String>> {
        let guard = self.data.read().await;
        let existing: HashSet<String> = guard.keys().cloned().collect();
        Ok(keys.difference(&existing).cloned().collect())
    }

    async fn status_counts(&self) -> Result<HashMap<DocStatus, usize>> {
        let guard = self.data.read().await;
        let mut counts: HashMap<DocStatus, usize> = HashMap::new();
        for record in guard.values() {
            *counts.entry(record.status.clone()).or_insert(0) += 1;
        }
        Ok(counts)
    }

    async fn status_counts_with_total(&self) -> Result<HashMap<DocStatus, usize>> {
        let mut counts = self.status_counts().await?;
        let total: usize = counts.values().copied().sum();
        counts.insert(DocStatus::ALL, total);
        Ok(counts)
    }

    async fn docs_by_status(
        &self,
        status: &DocStatus,
    ) -> Result<HashMap<String, DocProcessingStatus>> {
        let guard = self.data.read().await;
        Ok(guard
            .iter()
            .filter_map(|(id, record)| {
                if &record.status == status {
                    Some((id.clone(), record.to_status(id)))
                } else {
                    None
                }
            })
            .collect())
    }

    async fn docs_by_track_id(
        &self,
        track_id: &str,
    ) -> Result<HashMap<String, DocProcessingStatus>> {
        let guard = self.data.read().await;
        Ok(guard
            .iter()
            .filter_map(|(id, record)| {
                if record.track_id.as_deref() == Some(track_id) {
                    Some((id.clone(), record.to_status(id)))
                } else {
                    None
                }
            })
            .collect())
    }

    async fn docs_paginated(
        &self,
        status_filter: Option<&DocStatus>,
        page: usize,
        page_size: usize,
        sort_field: &str,
        sort_direction: &str,
    ) -> Result<(Vec<(String, DocProcessingStatus)>, usize)> {
        let page = page.max(1);
        let page_size = page_size.clamp(10, 200);
        let sort_field = match sort_field {
            "created_at" | "updated_at" | "id" | "file_path" => sort_field,
            _ => "updated_at",
        };
        let descending = matches!(sort_direction.to_ascii_lowercase().as_str(), "desc");

        let guard = self.data.read().await;
        let mut docs: Vec<(String, DocRecord)> = guard
            .iter()
            .filter_map(|(id, record)| {
                if let Some(filter) = status_filter {
                    if &record.status != filter {
                        return None;
                    }
                }
                Some((id.clone(), record.clone()))
            })
            .collect();

        docs.sort_by(|(id_a, rec_a), (id_b, rec_b)| {
            let key_a = Self::build_sort_key(rec_a, id_a, sort_field);
            let key_b = Self::build_sort_key(rec_b, id_b, sort_field);
            if descending {
                key_b.cmp(&key_a)
            } else {
                key_a.cmp(&key_b)
            }
        });

        let total = docs.len();
        let start = (page - 1) * page_size;
        let end = (start + page_size).min(total);
        let slice = if start >= total {
            &docs[0..0]
        } else {
            &docs[start..end]
        };

        let result = slice
            .iter()
            .map(|(id, record)| (id.clone(), record.to_status(id)))
            .collect();

        Ok((result, total))
    }

    async fn sync_if_dirty(&self) -> Result<()> {
        if !self.dirty.swap(false, AtomicOrdering::SeqCst) {
            return Ok(());
        }

        let snapshot = {
            let guard = self.data.read().await;
            guard.clone()
        };

        write_json_file(&self.file_path, &snapshot)
            .await
            .with_context(|| format!("failed to write doc status {}", self.final_namespace))?;
        Ok(())
    }
}
