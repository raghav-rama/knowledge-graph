use anyhow::{Context, Result};
use axum::{Router, extract::State, routing::get};
use serde::Deserialize;
use std::{env, net::SocketAddr, path::PathBuf, sync::Arc};
use tracing::{error, info};
use tracing_subscriber::EnvFilter;

const DEFAULT_CONFIG_PATH: &str = "config/app.yaml";

#[derive(Debug, Clone, Deserialize)]
struct AppConfig {
    server: ServerConfig,
    messages: MessagesConfig,
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

    let config = load_config().context("Failed to load application configuration")?;
    let addr_string = format!("{}:{}", config.server.host, config.server.port);
    let addr = addr_string
        .parse::<SocketAddr>()
        .with_context(|| format!("Invalid server address: {addr_string}"))?;
    info!(host = %config.server.host, port = config.server.port, "Loaded configuration");

    let shared_config = Arc::new(config);

    let app = Router::new()
        .route("/", get(handler))
        .with_state(shared_config.clone());

    let listener = tokio::net::TcpListener::bind(addr)
        .await
        .with_context(|| format!("Failed to bind TCP listener on {addr}"))?;
    info!(%addr, "Backend server listening");

    axum::serve(listener, app)
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

fn load_config() -> Result<AppConfig> {
    let path = config_path();
    let contents = std::fs::read_to_string(&path)
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

async fn handler(State(config): State<Arc<AppConfig>>) -> String {
    config.messages.greeting.clone()
}
