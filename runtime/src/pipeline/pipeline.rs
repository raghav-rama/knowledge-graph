use std::{
    collections::{HashMap, HashSet},
    path::PathBuf,
    sync::Arc,
};

use anyhow::{Result, anyhow};
use serde_json::{Value, json};
use tokio::sync::Mutex;
use tracing::{error, info, warn};
use uuid::Uuid;

use crate::storage::{
    DocProcessingStatus, DocStatus, DocStatusStorage, JsonKvStorage, KvStorage, StorageResult,
};

use super::{
    chunker::{ChunkConfig, Chunker},
    document_manager::DocumentManager,
    error_reporter::ErrorReporter,
    extractor::DocumentExtractor,
    status_service::{DocStatusService, PendingDocument},
    utils::{TiktokenTokenizer, Tokenizer, compute_mdhash_id},
};

#[derive(Clone)]
pub struct AppStorages {
    pub full_docs: Arc<JsonKvStorage>,
    pub text_chunks: Arc<JsonKvStorage>,
    pub full_entities: Arc<JsonKvStorage>,
    pub full_relations: Arc<JsonKvStorage>,
    pub llm_response_cache: Arc<JsonKvStorage>,
    pub doc_status: Arc<dyn DocStatusStorage>,
}

impl AppStorages {
    fn docs_storage(&self) -> Arc<dyn KvStorage> {
        self.full_docs.clone()
    }
}

#[derive(Debug, Clone)]
pub struct PipelineConfig {
    pub chunk_size: usize,
    pub chunk_overlap: usize,
    pub split_by_character: Option<String>,
    pub split_by_character_only: bool,
}

impl Default for PipelineConfig {
    fn default() -> Self {
        Self {
            chunk_size: 500,
            chunk_overlap: 50,
            split_by_character: None,
            split_by_character_only: false,
        }
    }
}

pub struct Pipeline {
    storages: Arc<AppStorages>,
    doc_manager: DocumentManager,
    chunker: Arc<dyn Chunker>,
    extractor: Arc<dyn DocumentExtractor>,
    status_service: DocStatusService,
    error_reporter: ErrorReporter,
    processing_lock: Arc<Mutex<()>>,
    config: PipelineConfig,
}

impl Pipeline {
    pub fn new(storages: Arc<AppStorages>, doc_manager: DocumentManager) -> Self {
        let tokenizer: Arc<dyn Tokenizer> =
            Arc::new(TiktokenTokenizer::new().expect("failed to initialize tokenizer"));
        let chunker = Arc::new(super::chunker::TokenizerChunker::new(tokenizer.clone()));
        let extractor = Arc::new(super::extractor::Utf8DocumentExtractor::new(
            doc_manager.file_repo(),
        ));
        let status_service =
            DocStatusService::new(storages.doc_status.clone(), storages.docs_storage());
        let error_reporter = ErrorReporter::new(storages.doc_status.clone());

        Self::with_dependencies(
            storages,
            doc_manager,
            PipelineConfig::default(),
            chunker,
            extractor,
            status_service,
            error_reporter,
        )
    }

    #[allow(clippy::too_many_arguments)]
    pub fn with_dependencies(
        storages: Arc<AppStorages>,
        doc_manager: DocumentManager,
        config: PipelineConfig,
        chunker: Arc<dyn Chunker>,
        extractor: Arc<dyn DocumentExtractor>,
        status_service: DocStatusService,
        error_reporter: ErrorReporter,
    ) -> Self {
        Self {
            storages,
            doc_manager,
            chunker,
            extractor,
            status_service,
            error_reporter,
            processing_lock: Arc::new(Mutex::new(())),
            config,
        }
    }

    pub fn document_manager(&self) -> &DocumentManager {
        &self.doc_manager
    }

    pub async fn enqueue_file(
        &self,
        file_path: PathBuf,
        track_id: Option<String>,
    ) -> Result<String> {
        let track_id = track_id.unwrap_or_else(|| generate_track_id("upload"));
        match self.extractor.extract(&file_path, &self.doc_manager).await {
            Ok(content) => {
                let doc_input = DocumentInput {
                    content,
                    file_path: file_path
                        .file_name()
                        .and_then(|n| n.to_str())
                        .unwrap_or_default()
                        .to_string(),
                };
                self.enqueue_documents(vec![doc_input], &track_id).await?;
                if let Err(err) = self.doc_manager.move_to_enqueued(&file_path).await {
                    warn!(error = %err, "failed moving file to enqueued directory");
                }
            }
            Err(err) => {
                error!(error = %err, "failed to extract file");
                self.error_reporter
                    .record(&file_path, &track_id, "file_extraction", &err)
                    .await?;
            }
        }

        Ok(track_id)
    }

    pub async fn process_queue(&self) -> Result<()> {
        let _guard = self.processing_lock.lock().await;

        let mut pending = self
            .storages
            .doc_status
            .docs_by_status(&DocStatus::PENDING)
            .await?;

        if pending.is_empty() {
            info!("no pending documents to process");
            return Ok(());
        }

        for (doc_id, status) in pending.drain() {
            if let Err(err) = self.process_document(&doc_id, &status).await {
                error!(error = %err, doc_id = %doc_id, "failed to process document");
                if let Err(status_err) = self
                    .status_service
                    .mark_failed(&doc_id, &status, &err)
                    .await
                {
                    error!(error = %status_err, doc_id = %doc_id, "failed to mark document as failed");
                }
            }
        }

        self.persist_all().await?;
        Ok(())
    }

    async fn process_document(&self, doc_id: &str, status: &DocProcessingStatus) -> Result<()> {
        let content_value = self
            .storages
            .full_docs
            .get_by_id(doc_id)
            .await?
            .ok_or_else(|| anyhow!("document content missing"))?;

        let content = content_value
            .get("content")
            .and_then(Value::as_str)
            .ok_or_else(|| anyhow!("document content field missing"))?;

        let chunk_config = ChunkConfig {
            max_tokens: self.config.chunk_size,
            overlap_tokens: self.config.chunk_overlap,
            split_by_character: self.config.split_by_character.clone(),
            split_by_character_only: self.config.split_by_character_only,
        };

        let chunks = self.chunker.chunk(content, &chunk_config)?;
        if chunks.is_empty() {
            warn!(doc_id = %doc_id, "no chunks created for document");
        }

        let chunk_ids: Vec<String> = chunks.iter().map(|chunk| chunk.id.clone()).collect();
        self.status_service
            .mark_processing(doc_id, status, &chunk_ids)
            .await?;

        let chunk_map: HashMap<String, Value> = chunks
            .into_iter()
            .map(|chunk| {
                let obj = json!({
                    "content": chunk.content,
                    "full_doc_id": doc_id,
                    "chunk_order_index": chunk.order,
                    "file_path": status.file_path.clone().unwrap_or_default(),
                    "tokens": chunk.token_count,
                });
                (chunk.id, obj)
            })
            .collect();

        if !chunk_map.is_empty() {
            self.storages.text_chunks.upsert(chunk_map).await?;
        }

        self.persist_all().await?;

        self.status_service
            .mark_processed(doc_id, status, &chunk_ids)
            .await?;

        Ok(())
    }

    async fn enqueue_documents(&self, docs: Vec<DocumentInput>, track_id: &str) -> Result<()> {
        if docs.is_empty() {
            return Ok(());
        }

        let mut unique_contents: HashMap<String, String> = HashMap::new();
        for doc in docs {
            let cleaned = sanitize_text(&doc.content);
            if cleaned.is_empty() {
                continue;
            }
            unique_contents
                .entry(cleaned)
                .or_insert_with(|| doc.file_path.clone());
        }

        if unique_contents.is_empty() {
            return Ok(());
        }

        let mut contents: HashMap<String, (String, String)> = HashMap::new();
        for (content, path) in unique_contents {
            let doc_id = compute_mdhash_id(&content, "doc-");
            contents.insert(doc_id, (content, path));
        }

        let doc_ids: HashSet<String> = contents.keys().cloned().collect();
        let unique_ids = self.status_service.filter_new_ids(&doc_ids).await?;

        if unique_ids.is_empty() {
            warn!("no new documents to enqueue");
            return Ok(());
        }

        let now = chrono::Utc::now().to_rfc3339();
        let mut pending = Vec::new();

        for doc_id in unique_ids {
            if let Some((content, path)) = contents.remove(&doc_id) {
                let summary = summarize_content(&content);
                let length = content.chars().count() as i64;
                pending.push(PendingDocument {
                    id: doc_id,
                    content,
                    summary,
                    length,
                    file_path: path,
                    track_id: track_id.to_string(),
                    created_at: now.clone(),
                });
            }
        }

        self.status_service.enqueue_pending(pending).await?;
        self.persist_all().await?;
        Ok(())
    }

    async fn persist_all(&self) -> StorageResult<()> {
        self.storages.full_docs.sync_if_dirty().await?;
        self.storages.text_chunks.sync_if_dirty().await?;
        self.storages.full_entities.sync_if_dirty().await?;
        self.storages.full_relations.sync_if_dirty().await?;
        self.storages.llm_response_cache.sync_if_dirty().await?;
        self.storages.doc_status.sync_if_dirty().await?;
        Ok(())
    }
}

fn generate_track_id(prefix: &str) -> String {
    format!("{}-{}", prefix, Uuid::new_v4())
}

fn sanitize_text(input: &str) -> String {
    input.replace('\r', "").trim().to_string()
}

fn summarize_content(content: &str) -> String {
    const MAX_LEN: usize = 200;
    let trimmed = content.trim();
    if trimmed.len() <= MAX_LEN {
        trimmed.to_string()
    } else {
        format!("{}…", &trimmed[..MAX_LEN])
    }
}

#[derive(Clone)]
struct DocumentInput {
    content: String,
    file_path: String,
}
