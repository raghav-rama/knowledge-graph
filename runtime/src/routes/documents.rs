use std::sync::Arc;

use axum::{
    Json, Router,
    extract::{Multipart, State},
    http::StatusCode,
    routing::{get, post},
};
use rand::{Rng, rng, seq::SliceRandom};
use serde::Serialize;
use tokio::fs;
use tracing::{debug, error, info, warn};

use crate::{AppState, pipeline::scheduler::Job};

#[derive(Serialize)]
struct InsertResponse {
    status: String,
    message: String,
    track_id: String,
}

#[derive(Serialize)]
struct DocumentListResponse {
    total: usize,
    documents: Vec<DocumentSummary>,
}

#[derive(Serialize)]
struct DocumentSummary {
    id: String,
    summary: String,
    status: String,
    length: i64,
    chunks: usize,
    created_at: Option<String>,
    updated_at: Option<String>,
    file_path: Option<String>,
    track_id: Option<String>,
}

pub fn document_routes() -> Router<Arc<AppState>> {
    Router::new()
        .route("/documents/upload", post(upload_to_input_dir))
        .route("/documents", get(list_documents))
}

async fn list_documents(
    State(state): State<Arc<AppState>>,
) -> Result<Json<DocumentListResponse>, (StatusCode, String)> {
    let (records, total) = state
        .storages
        .doc_status
        .docs_paginated(None, 1, 200, "updated_at", "desc")
        .await
        .map_err(|err| {
            (
                StatusCode::INTERNAL_SERVER_ERROR,
                format!("failed to load documents: {err}"),
            )
        })?;

    let documents = records
        .into_iter()
        .map(|(id, status)| {
            let summary = status
                .content_summary
                .clone()
                .or_else(|| status.file_path.clone())
                .unwrap_or_else(|| "No summary available".to_string());

            DocumentSummary {
                id: id.clone(),
                summary,
                status: map_status(&status.status),
                length: status.content_length.unwrap_or_default(),
                chunks: status
                    .chunks_list
                    .as_ref()
                    .map(|chunks| chunks.len())
                    .unwrap_or_default(),
                created_at: status.created_at.clone(),
                updated_at: status.updated_at.clone(),
                file_path: status.file_path.clone(),
                track_id: status.track_id.clone(),
            }
        })
        .collect();

    Ok(Json(DocumentListResponse { total, documents }))
}

async fn upload_to_input_dir(
    State(state): State<Arc<AppState>>,
    mut multipart: Multipart,
) -> Result<Json<InsertResponse>, (StatusCode, String)> {
    let mut file_bytes: Option<Vec<u8>> = None;
    let mut original_filename: Option<String> = None;

    while let Some(field) = multipart.next_field().await.map_err(|err| {
        (
            StatusCode::BAD_REQUEST,
            format!("invalid multipart payload: {err}"),
        )
    })? {
        if field.name() == Some("file") {
            original_filename = field.file_name().map(|name| name.to_string());
            let data = field.bytes().await.map_err(|err| {
                (
                    StatusCode::BAD_REQUEST,
                    format!("failed to read upload field: {err}"),
                )
            })?;
            file_bytes = Some(data.to_vec());
            break;
        }
    }

    let file_bytes = file_bytes.ok_or_else(|| {
        (
            StatusCode::BAD_REQUEST,
            "missing file field in multipart payload".to_string(),
        )
    })?;

    let original_filename = original_filename.ok_or_else(|| {
        (
            StatusCode::BAD_REQUEST,
            "uploaded file missing filename".to_string(),
        )
    })?;

    let doc_manager = state.pipeline.document_manager().clone();

    let safe_filename = doc_manager
        .sanitize_filename(&original_filename)
        .map_err(|err| {
            (
                StatusCode::BAD_REQUEST,
                format!("invalid filename '{}': {err}", original_filename),
            )
        })?;

    if !doc_manager.is_supported_file(&safe_filename) {
        return Err((
            StatusCode::BAD_REQUEST,
            format!(
                "unsupported file type. supported types: {:?}",
                crate::SUPPORTED_EXTENSIONS
            ),
        ));
    }
    let scheduler = state.scheduler.clone();
    // let mut guard = scheduler.queue.lock().await;

    // let job = Job::new(String::from(
    //     "doc-0b848f200e91a3de05babf664421ca6f1d57044f2868dd17f397d36f02f12c76",
    // ));
    // let result = guard.enqueue(job.job_id.clone(), job);
    // match result {
    //     Ok(job_id) => debug!("Enqueued {}", job_id),
    //     Err(err) => error!(error=%err, "Error"),
    // }
    // drop(guard);

    let existing_doc = state
        .storages
        .doc_status
        .get_doc_by_file_path(&safe_filename)
        .await
        .map_err(|err| {
            (
                StatusCode::INTERNAL_SERVER_ERROR,
                format!("failed to query document status: {err}"),
            )
        })?;

    if let Some(status) = existing_doc {
        let status_label = format!("{:?}", status.status);
        return Ok(Json(InsertResponse {
            status: "duplicated".to_string(),
            message: format!(
                "File '{}' already exists in document storage (Status: {}).",
                safe_filename, status_label
            ),
            track_id: String::new(),
        }));
    }

    let target_dir = doc_manager.input_dir();
    let file_path = target_dir.join(&safe_filename);

    if file_path.exists() {
        return Ok(Json(InsertResponse {
            status: "duplicated".to_string(),
            message: format!(
                "File '{}' already exists in the input directory.",
                safe_filename
            ),
            track_id: String::new(),
        }));
    }

    fs::write(&file_path, &file_bytes).await.map_err(|err| {
        (
            StatusCode::INTERNAL_SERVER_ERROR,
            format!("failed to persist uploaded file: {err}"),
        )
    })?;

    let pipeline = state.pipeline.clone();
    let enqueue_path = file_path.clone();
    let track_id = pipeline
        .enqueue_file(enqueue_path, None)
        .await
        .map_err(|err| {
            (
                StatusCode::INTERNAL_SERVER_ERROR,
                format!("failed to enqueue document: {err}"),
            )
        })?;

    let background_pipeline = pipeline.clone();

    tokio::spawn(async move {
        if let Err(err) = background_pipeline.enqueue_pending_docs(scheduler).await {
            warn!(error = %err, "background pipeline processing failed");
        }
    });

    info!(filename = %safe_filename, track_id = %track_id, "file uploaded successfully");

    Ok(Json(InsertResponse {
        status: "success".to_string(),
        message: format!(
            "File '{}' uploaded successfully. Processing will continue in background.",
            safe_filename
        ),
        track_id,
    }))
}

fn map_status(status: &crate::storage::DocStatus) -> String {
    use crate::storage::DocStatus;
    match status {
        DocStatus::PROCESSED => "Completed".to_string(),
        DocStatus::PROCESSING => "Processing".to_string(),
        DocStatus::PENDING => "Pending".to_string(),
        DocStatus::FAILED => "Failed".to_string(),
        DocStatus::ALL => "All".to_string(),
    }
}

fn random_string(len: usize) -> String {
    let ss = b"ABCDEF1234567890";
    let mut rng = rng();
    let len = ss.len();
    (0..len)
        .map(|_| {
            let idx = rng.random_range(0..len);
            ss[idx] as char
        })
        .collect()
}
