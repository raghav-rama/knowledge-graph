use std::{
    collections::{HashMap, HashSet},
    path::{Path, PathBuf},
    sync::Arc,
};

use anyhow::{Context, Result, anyhow};
use serde_json::{Value, json};
use tokio::{fs, sync::Mutex};
use tracing::{error, info, warn};
use uuid::Uuid;

use crate::storage::{
    DocProcessingStatus, DocStatus, DocStatusStorage, JsonKvStorage, KvStorage, StorageResult,
};

#[derive(Clone)]
pub struct DocumentManager {
    base_input_dir: PathBuf,
    workspace: Option<String>,
    supported_extensions: HashSet<String>,
}

impl DocumentManager {
    pub async fn new<P>(
        input_dir: P,
        workspace: Option<String>,
        supported_extensions: &[&str],
    ) -> Result<Self>
    where
        P: AsRef<Path>,
    {
        let mut extensions = HashSet::new();
        for ext in supported_extensions {
            extensions.insert(normalize_extension(ext));
        }

        let base_input_dir = input_dir.as_ref().to_path_buf();
        let input_dir = if let Some(ws) = workspace.as_deref() {
            base_input_dir.join(ws)
        } else {
            base_input_dir.clone()
        };

        fs::create_dir_all(&input_dir).await.with_context(|| {
            format!(
                "failed to create input directory at {}",
                input_dir.display()
            )
        })?;

        Ok(Self {
            base_input_dir,
            workspace,
            supported_extensions: extensions,
        })
    }

    pub fn input_dir(&self) -> PathBuf {
        if let Some(ws) = self.workspace.as_deref() {
            self.base_input_dir.join(ws)
        } else {
            self.base_input_dir.clone()
        }
    }

    pub fn is_supported_file(&self, filename: &str) -> bool {
        let ext = Path::new(filename)
            .extension()
            .and_then(|os| os.to_str())
            .map(normalize_extension);
        match ext {
            Some(ext) => self.supported_extensions.contains(&ext),
            None => false,
        }
    }

    pub fn sanitize_filename(&self, raw: &str) -> Result<String> {
        let trimmed = raw.trim();
        if trimmed.is_empty() {
            return Err(anyhow!("filename cannot be empty"));
        }

        if trimmed.contains("..") || trimmed.contains('/') || trimmed.contains('\\') {
            return Err(anyhow!("invalid filename"));
        }

        Ok(trimmed.to_string())
    }

    pub async fn path_is_duplicate(&self, filename: &str) -> bool {
        self.input_dir().join(filename).exists()
    }

    pub async fn move_to_enqueued(&self, file_path: &Path) -> Result<PathBuf> {
        let parent = file_path
            .parent()
            .ok_or_else(|| anyhow!("file has no parent directory"))?;
        let enqueued_dir = parent.join("__enqueued__");
        fs::create_dir_all(&enqueued_dir).await.with_context(|| {
            format!(
                "failed to create enqueued dir at {}",
                enqueued_dir.display()
            )
        })?;

        let unique_name = unique_filename(
            &enqueued_dir,
            file_path
                .file_name()
                .unwrap_or_default()
                .to_string_lossy()
                .as_ref(),
        );
        let target = enqueued_dir.join(&unique_name);
        fs::rename(file_path, &target)
            .await
            .with_context(|| format!("failed to move {} to enqueued dir", file_path.display()))?;
        Ok(target)
    }
}

fn normalize_extension(ext: &str) -> String {
    if let Some(stripped) = ext.strip_prefix('.') {
        stripped.to_ascii_lowercase()
    } else {
        ext.to_ascii_lowercase()
    }
}

fn unique_filename(dir: &Path, original: &str) -> String {
    let mut candidate = dir.join(original);
    if !candidate.exists() {
        return original.to_string();
    }

    let mut counter = 1usize;
    let (stem, ext) = match Path::new(original).file_stem().and_then(|s| s.to_str()) {
        Some(stem) => {
            let ext = Path::new(original)
                .extension()
                .and_then(|e| e.to_str())
                .unwrap_or("");
            (stem.to_string(), ext.to_string())
        }
        None => (original.to_string(), String::new()),
    };

    loop {
        let candidate_name = if ext.is_empty() {
            format!("{}_{}", stem, counter)
        } else {
            format!("{}_{}.{}", stem, counter, ext)
        };
        candidate = dir.join(&candidate_name);
        if !candidate.exists() {
            return candidate_name;
        }
        counter += 1;
    }
}

#[derive(Clone)]
pub struct Pipeline {
    storages: Arc<AppStorages>,
    doc_manager: DocumentManager,
    processing_lock: Arc<Mutex<()>>,
    chunk_size: usize,
    chunk_overlap: usize,
}

#[derive(Clone)]
pub struct AppStorages {
    pub full_docs: Arc<JsonKvStorage>,
    pub text_chunks: Arc<JsonKvStorage>,
    pub full_entities: Arc<JsonKvStorage>,
    pub full_relations: Arc<JsonKvStorage>,
    pub llm_response_cache: Arc<JsonKvStorage>,
    pub doc_status: Arc<dyn DocStatusStorage>,
}

impl Pipeline {
    pub fn new(storages: Arc<AppStorages>, doc_manager: DocumentManager) -> Self {
        Self {
            storages,
            doc_manager,
            processing_lock: Arc::new(Mutex::new(())),
            chunk_size: 500,
            chunk_overlap: 50,
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
        match self.extract_file(&file_path).await {
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
                self.doc_manager
                    .move_to_enqueued(&file_path)
                    .await
                    .map_err(|err| {
                        warn!(error = %err, "failed moving file to enqueued directory");
                        err
                    })
                    .ok();
            }
            Err(err) => {
                error!(error = %err, "failed to extract file");
                self.register_error(file_path, &track_id, "file_extraction", &err)
                    .await?;
            }
        }

        Ok(track_id)
    }

    async fn extract_file(&self, file_path: &Path) -> Result<String> {
        let bytes = fs::read(file_path)
            .await
            .with_context(|| format!("failed to read file {}", file_path.display()))?;
        if bytes.is_empty() {
            return Err(anyhow!("file content is empty"));
        }

        let extension = file_path
            .extension()
            .and_then(|ext| ext.to_str())
            .map(normalize_extension)
            .unwrap_or_default();

        let text =
            String::from_utf8(bytes.clone()).map_err(|_| anyhow!("file is not valid UTF-8"))?;
        if text.trim().is_empty() {
            return Err(anyhow!("file contains only whitespace"));
        }

        if !self.doc_manager.supported_extensions.contains(&extension) {
            return Err(anyhow!("unsupported file type"));
        }

        Ok(text)
    }

    async fn register_error(
        &self,
        file_path: PathBuf,
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
            content_summary: Some(format!("{} failed for {}", error_type, filename)),
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
        let doc_id = compute_mdhash_id(&format!("error-{}-{}", track_id, filename), "error-");
        let mut payload = HashMap::new();
        payload.insert(doc_id, error_doc);
        self.storages.doc_status.upsert(payload).await?;
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
        let unique_ids = self.storages.doc_status.filter_keys(&doc_ids).await?;

        if unique_ids.is_empty() {
            warn!("no new documents to enqueue");
            return Ok(());
        }

        let now = chrono::Utc::now().to_rfc3339();
        let mut status_payload = HashMap::new();
        let mut docs_payload = HashMap::new();

        for doc_id in unique_ids {
            if let Some((content, path)) = contents.remove(&doc_id) {
                let summary = summarize_content(&content);
                let length = content.chars().count() as i64;
                docs_payload.insert(doc_id.clone(), json!({ "content": content }));
                status_payload.insert(
                    doc_id.clone(),
                    DocProcessingStatus {
                        id: Some(doc_id.clone()),
                        status: DocStatus::PENDING,
                        content_summary: Some(summary),
                        content_length: Some(length),
                        created_at: Some(now.clone()),
                        updated_at: Some(now.clone()),
                        file_path: Some(path),
                        track_id: Some(track_id.to_string()),
                        chunks_list: Some(vec![]),
                        metadata: None,
                        error_msg: None,
                    },
                );
            }
        }

        if docs_payload.is_empty() {
            return Ok(());
        }

        self.storages.full_docs.upsert(docs_payload).await?;
        self.storages.doc_status.upsert(status_payload).await?;
        self.persist_all().await?;
        Ok(())
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
                let now = chrono::Utc::now().to_rfc3339();
                let mut payload = HashMap::new();
                payload.insert(
                    doc_id.clone(),
                    DocProcessingStatus {
                        id: Some(doc_id.clone()),
                        status: DocStatus::FAILED,
                        content_summary: status.content_summary.clone(),
                        content_length: status.content_length,
                        created_at: status.created_at.clone(),
                        updated_at: Some(now),
                        file_path: status.file_path.clone(),
                        track_id: status.track_id.clone(),
                        chunks_list: Some(vec![]),
                        metadata: None,
                        error_msg: Some(err.to_string()),
                    },
                );
                let _ = self.storages.doc_status.upsert(payload).await;
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
            .ok_or_else(|| anyhow!("document content malformed"))?;

        let chunks = self.build_chunks(content);
        if chunks.is_empty() {
            return Err(anyhow!("document produced no chunks"));
        }

        let chunk_ids: Vec<String> = chunks.iter().map(|chunk| chunk.id.clone()).collect();

        let now = chrono::Utc::now().to_rfc3339();
        let mut processing_payload = HashMap::new();
        processing_payload.insert(
            doc_id.to_string(),
            DocProcessingStatus {
                id: Some(doc_id.to_string()),
                status: DocStatus::PROCESSING,
                content_summary: status.content_summary.clone(),
                content_length: status.content_length,
                created_at: status.created_at.clone(),
                updated_at: Some(now.clone()),
                file_path: status.file_path.clone(),
                track_id: status.track_id.clone(),
                chunks_list: Some(chunk_ids.clone()),
                metadata: None,
                error_msg: None,
            },
        );
        self.storages.doc_status.upsert(processing_payload).await?;

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

        self.storages.text_chunks.upsert(chunk_map.clone()).await?;
        self.persist_all().await?;

        let mut processed_payload = HashMap::new();
        processed_payload.insert(
            doc_id.to_string(),
            DocProcessingStatus {
                id: Some(doc_id.to_string()),
                status: DocStatus::PROCESSED,
                content_summary: status.content_summary.clone(),
                content_length: status.content_length,
                created_at: status.created_at.clone(),
                updated_at: Some(chrono::Utc::now().to_rfc3339()),
                file_path: status.file_path.clone(),
                track_id: status.track_id.clone(),
                chunks_list: Some(chunk_ids),
                metadata: None,
                error_msg: None,
            },
        );
        self.storages.doc_status.upsert(processed_payload).await?;
        Ok(())
    }

    fn build_chunks(&self, content: &str) -> Vec<Chunk> {
        let words: Vec<&str> = content.split_whitespace().collect();
        if words.is_empty() {
            return Vec::new();
        }

        let mut chunks = Vec::new();
        let mut start = 0usize;
        let chunk_word_size = self.chunk_size;
        let overlap = self.chunk_overlap.min(chunk_word_size);

        while start < words.len() {
            let end = (start + chunk_word_size).min(words.len());
            let chunk_words = &words[start..end];
            let chunk_text = chunk_words.join(" ");
            let chunk_id = compute_mdhash_id(&chunk_text, "chunk-");
            chunks.push(Chunk {
                id: chunk_id,
                content: chunk_text,
                order: chunks.len(),
                token_count: chunk_words.len() as i64,
            });

            if end == words.len() {
                break;
            }

            start = end.saturating_sub(overlap);
        }

        chunks
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

fn compute_mdhash_id(content: &str, prefix: &str) -> String {
    use sha2::{Digest, Sha256};
    let mut hasher = Sha256::new();
    hasher.update(content.as_bytes());
    let digest = hasher.finalize();
    format!("{}{:x}", prefix, digest)
}

#[derive(Clone)]
struct DocumentInput {
    content: String,
    file_path: String,
}

struct Chunk {
    id: String,
    content: String,
    order: usize,
    token_count: i64,
}
