#![allow(dead_code)]
#![allow(unused_imports)]
#![allow(unused_variables)]

use anyhow::{Context, Result};
use axum::{Router, extract::State, http::StatusCode, routing::get};
use dotenvy::dotenv;
use serde::Deserialize;
use std::{env, net::SocketAddr, path::PathBuf, sync::Arc};
use tokio::{
    fs,
    net::TcpListener,
    signal,
    sync::mpsc::{self as mpsc, Receiver, Sender},
};
use tracing::{error, info, warn};
use tracing_subscriber::EnvFilter;

mod ai;
mod pipeline;
mod routes;
mod storage;
use ai::responses::ResponsesClient;
use pipeline::{
    AppStorages, DocumentManager, Pipeline,
    scheduler::{JobDispatch, JobResult},
};
use storage::{
    DocStatusStorage, KvStorage, StorageManager, StoragesStatus,
    json_doc_status::{JsonDocStatusConfig, JsonDocStatusStorage},
    json_kv::{JsonKvStorage, JsonKvStorageConfig},
};

const DEFAULT_CONFIG_PATH: &str = "config/app.yaml";
pub(crate) const SUPPORTED_EXTENSIONS: &[&str] = &[".txt", ".md", ".json", ".csv"];

#[derive(Debug, Clone, Deserialize)]
struct AppConfig {
    server: ServerConfig,
    working_dir: String,
}

#[derive(Clone)]
pub(crate) struct AppState {
    config: Arc<AppConfig>,
    storages: Arc<AppStorages>,
    pipeline: Arc<Pipeline>,
    storages_status: StoragesStatus,
    ai_client: Arc<ResponsesClient>,
}

#[derive(Debug, Clone, Deserialize)]
struct ServerConfig {
    host: String,
    port: u16,
}

#[tokio::main]
async fn main() {
    if let Err(err) = run().await {
        error!(error = %err, "Backend crashed");
        eprintln!("Backend crashed: {err}");
    }
}

async fn run() -> Result<()> {
    init_tracing();
    dotenv().with_context(|| "Problem loading .env file")?;
    let api_key = env::var("OPENAI_API_KEY").context("openai aapi key not set")?;

    let (work_tx, mut work_rx) = mpsc::channel::<JobDispatch>(100);
    let (job_result_tx, mut job_result_rx) = mpsc::channel::<JobResult>(100);
    let config = load_config()
        .await
        .context("Failed to load application configuration")?;
    let working_dir = PathBuf::from(&config.working_dir);
    let workspace = env::var("WORKSPACE")
        .ok()
        .map(|value| value.trim().to_string())
        .filter(|value| !value.is_empty());

    let full_docs = Arc::new(JsonKvStorage::new(JsonKvStorageConfig {
        working_dir: working_dir.clone(),
        namespace: "full_docs".into(),
        workspace: workspace.clone(),
    }));

    let text_chunks = Arc::new(JsonKvStorage::new(JsonKvStorageConfig {
        working_dir: working_dir.clone(),
        namespace: "text_chunks".into(),
        workspace: workspace.clone(),
    }));

    let full_entities = Arc::new(JsonKvStorage::new(JsonKvStorageConfig {
        working_dir: working_dir.clone(),
        namespace: "full_entities".into(),
        workspace: workspace.clone(),
    }));

    let full_relations = Arc::new(JsonKvStorage::new(JsonKvStorageConfig {
        working_dir: working_dir.clone(),
        namespace: "full_relations".into(),
        workspace: workspace.clone(),
    }));

    let llm_response_cache = Arc::new(JsonKvStorage::new(JsonKvStorageConfig {
        working_dir: working_dir.clone(),
        namespace: "llm_response_cache".into(),
        workspace: workspace.clone(),
    }));

    let doc_status_storage = Arc::new(JsonDocStatusStorage::new(JsonDocStatusConfig {
        working_dir: working_dir.clone(),
        namespace: "doc_status".into(),
        workspace: workspace.clone(),
    }));

    let mut storage_manager = StorageManager::new();
    storage_manager.register_kv(full_docs.clone());
    storage_manager.register_kv(text_chunks.clone());
    storage_manager.register_kv(full_entities.clone());
    storage_manager.register_kv(full_relations.clone());
    storage_manager.register_kv(llm_response_cache.clone());
    storage_manager.register_doc_status(doc_status_storage.clone());
    storage_manager.initialize_all().await?;

    let storages = Arc::new(AppStorages {
        full_docs,
        text_chunks,
        full_entities,
        full_relations,
        llm_response_cache,
        doc_status: doc_status_storage.clone(),
    });

    let document_manager = DocumentManager::new(
        working_dir.join("input"),
        workspace.clone(),
        SUPPORTED_EXTENSIONS,
    )
    .await?;

    let ai_client = Arc::new(ResponsesClient::new(api_key, None));

    let pipeline = Arc::new(Pipeline::new(
        storages.clone(),
        document_manager,
        ai_client.clone(),
    ));

    let state = Arc::new(AppState {
        config: Arc::new(config.clone()),
        storages,
        pipeline,
        storages_status: storage_manager.status(),
        ai_client,
    });

    let addr_string = format!("{}:{}", config.server.host, config.server.port);
    let addr = addr_string
        .parse::<SocketAddr>()
        .with_context(|| format!("Invalid server address: {addr_string}"))?;
    info!(host = %config.server.host, port = config.server.port, "Loaded configuration");

    let app = Router::new()
        .route("/", get(handler))
        .route("/health", get(health))
        .merge(routes::document_routes())
        .merge(routes::graph_routes())
        .with_state(state);

    let listener = TcpListener::bind(addr)
        .await
        .with_context(|| format!("Failed to bind TCP listener on {addr}"))?;
    info!(%addr, "Backend server listening");

    let server_result = axum::serve(listener, app)
        .with_graceful_shutdown(shutdown_signal())
        .await;

    if let Err(err) = storage_manager.finalize_all().await {
        warn!(error = %err, "Failed to finalize storages");
    }

    server_result.context("Server encountered a fatal error")?;
    Ok(())
}

fn init_tracing() {
    let env_filter = EnvFilter::try_from_default_env().unwrap_or_else(|_| EnvFilter::new("info"));
    tracing_subscriber::fmt()
        .with_env_filter(env_filter)
        .with_target(false)
        .compact()
        .init();
}

async fn load_config() -> Result<AppConfig> {
    let path = config_path();
    let contents = fs::read_to_string(&path)
        .await
        .with_context(|| format!("Failed to read config file at {}", path.display()))?;
    let config: AppConfig = serde_yaml::from_str(&contents)
        .with_context(|| format!("Failed to parse config file at {}", path.display()))?;
    info!(path = %path.display(), "Configuration loaded from disk");
    Ok(config)
}

fn config_path() -> PathBuf {
    env::var("APP_CONFIG_PATH")
        .map(PathBuf::from)
        .unwrap_or_else(|_| PathBuf::from(DEFAULT_CONFIG_PATH))
}

async fn handler(State(state): State<Arc<AppState>>) -> Result<String, StatusCode> {
    let _docs = state
        .storages
        .doc_status
        .docs_paginated(None, 1, 10, "updated_at", "desc")
        .await
        .map_err(|_| StatusCode::INTERNAL_SERVER_ERROR)?;
    Ok("Connected".to_owned())
}

#[inline]
async fn health() -> &'static str {
    "ok"
}

async fn shutdown_signal() {
    let ctrl_c = async {
        if let Err(err) = signal::ctrl_c().await {
            error!(error = %err, "Failed to listen for Ctrl+C");
        }
    };

    #[cfg(unix)]
    let terminate = async {
        use tokio::signal::unix::{SignalKind, signal};

        match signal(SignalKind::terminate()) {
            Ok(mut stream) => {
                if stream.recv().await.is_some() {
                    info!("Received SIGTERM");
                }
            }
            Err(err) => warn!(error = %err, "Failed to install SIGTERM handler"),
        }
    };

    #[cfg(not(unix))]
    let terminate = std::future::pending::<()>();

    tokio::select! {
        _ = ctrl_c => {
            info!("Received termination signal (Ctrl+C)");
        }
        _ = terminate => {
            info!("Received termination signal (SIGTERM)");
        }
    }
}
