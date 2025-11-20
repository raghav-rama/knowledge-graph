use crate::AppState;
use axum::{
    Json, Router,
    body::Body,
    extract::{Query, State},
    http::{Response, StatusCode},
    response::IntoResponse,
    routing::get,
};
use serde::Deserialize;
use std::sync::Arc;
use tokio_util::io::ReaderStream;

use tokio::fs::File;
use tokio::io::AsyncReadExt;

pub fn download_routes() -> Router<Arc<AppState>> {
    Router::new().route("/download", get(download_handler))
}

#[derive(Deserialize)]
struct DownloadFileQueryParams {
    moniker: FileMoniker,
}

#[derive(Deserialize)]
enum FileMoniker {
    Entities,
    Relations,
}

async fn download_handler(
    State(state): State<Arc<AppState>>,
    Query(params): Query<DownloadFileQueryParams>,
) -> Result<Response<Body>, (StatusCode, String)> {
    let file_moniker = params.moniker;

    let response = match file_moniker {
        FileMoniker::Entities => {
            let file = File::open("/opt/runtime/pgv-data-test/kv_store_full_entities.json")
                .await
                .map_err(|err| {
                    (
                        StatusCode::NOT_FOUND,
                        format!("File does not exists: {}", err),
                    )
                })?;
            let stream = ReaderStream::new(file);
            let response = Response::builder()
                .header(
                    "Content-Disposition",
                    "attachment;filename=full-entities.json",
                )
                .header("Content-Type", "application/octet-stream")
                .body(Body::from_stream(stream))
                .map_err(|err| {
                    (
                        StatusCode::INTERNAL_SERVER_ERROR,
                        format!("Error in sending response {}", err),
                    )
                });
            response
        }
        FileMoniker::Relations => {
            let file = File::open("/opt/runtime/pgv-data-test/kv_store_full_relations.json")
                .await
                .map_err(|err| {
                    (
                        StatusCode::NOT_FOUND,
                        format!("File does not exists: {}", err),
                    )
                })?;
            let stream = ReaderStream::new(file);
            let response = Response::builder()
                .header(
                    "Content-Disposition",
                    "attachment;filename=full-relations.json",
                )
                .header("Content-Type", "application/octet-stream")
                .body(Body::from_stream(stream))
                .map_err(|err| {
                    (
                        StatusCode::INTERNAL_SERVER_ERROR,
                        format!("Error in sending response {}", err),
                    )
                });
            response
        } // _ => Err((
          //     StatusCode::BAD_REQUEST,
          //     String::from("Only Entities or Relations allowed"),
          // )),
    };

    response
}
