use anyhow::{Context, Result};
use axum::{Router, extract::State, http::StatusCode, routing::get};
use serde::Deserialize;
use std::{env, net::SocketAddr, path::PathBuf, sync::Arc};
use tokio::{fs, net::TcpListener, signal};
use tracing::{error, info, warn};
use tracing_subscriber::EnvFilter;

mod storage;

use storage::{
    DocStatusStorage, KvStorage,
    json_doc_status::{JsonDocStatusConfig, JsonDocStatusStorage},
    json_kv::{JsonKvStorage, JsonKvStorageConfig},
};

const DEFAULT_CONFIG_PATH: &str = "config/app.yaml";

#[derive(Debug, Clone, Deserialize)]
struct AppConfig {
    server: ServerConfig,
    messages: MessagesConfig,
    working_dir: String,
}

#[derive(Clone)]
struct AppState {
    config: Arc<AppConfig>,
    kv_storage: Arc<JsonKvStorage>,
    doc_status_storage: Arc<JsonDocStatusStorage>,
}

#[derive(Debug, Clone, Deserialize)]
struct ServerConfig {
    host: String,
    port: u16,
}

#[derive(Debug, Clone, Deserialize)]
struct MessagesConfig {
    greeting: String,
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

    let config = load_config()
        .await
        .context("Failed to load application configuration")?;
    let working_dir = PathBuf::from("data");
    let kv_storage = Arc::new(JsonKvStorage::new(JsonKvStorageConfig {
        working_dir: working_dir.clone(),
        namespace: "text_chunks".into(),
        workspace: Some("default".into()),
    }));
    kv_storage.initialize().await?;

    let doc_status_storage = Arc::new(JsonDocStatusStorage::new(JsonDocStatusConfig {
        working_dir,
        namespace: "doc_status".into(),
        workspace: None,
    }));
    doc_status_storage.initialize().await?;

    let state = Arc::new(AppState {
        config: Arc::new(config.clone()),
        kv_storage,
        doc_status_storage,
    });

    let addr_string = format!("{}:{}", config.server.host, config.server.port);
    let addr = addr_string
        .parse::<SocketAddr>()
        .with_context(|| format!("Invalid server address: {addr_string}"))?;
    info!(host = %config.server.host, port = config.server.port, "Loaded configuration");

    let app = Router::new()
        .route("/", get(handler))
        .route("/health", get(health))
        .with_state(state);

    let listener = TcpListener::bind(addr)
        .await
        .with_context(|| format!("Failed to bind TCP listener on {addr}"))?;
    info!(%addr, "Backend server listening");

    axum::serve(listener, app)
        .with_graceful_shutdown(shutdown_signal())
        .await
        .context("Server encountered a fatal error")?;
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
    let docs = state
        .doc_status_storage
        .docs_paginated(None, 1, 10, "updated_at", "desc")
        .await
        .map_err(|_| StatusCode::INTERNAL_SERVER_ERROR)?;
    info!("HIT - {:?}", docs);
    Ok("docs.0".to_owned())
}

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
