use std::sync::Arc;

use axum::{
    Json, Router,
    extract::{Multipart, State},
    http::StatusCode,
    response::IntoResponse,
    routing::post,
};
use serde::Serialize;
use tokio::fs;
use tracing::{error, info, warn};

use crate::AppState;

#[derive(Serialize)]
struct InsertResponse {
    status: String,
    message: String,
    track_id: String,
}

pub fn document_routes() -> Router<Arc<AppState>> {
    Router::new().route("/documents/upload", post(upload_to_input_dir))
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
        if let Err(err) = background_pipeline.process_queue().await {
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
