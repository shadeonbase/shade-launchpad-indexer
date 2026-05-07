use anyhow::{Context, Result};
use serde::Deserialize;
use std::path::{Path, PathBuf};

#[derive(Debug, Clone, Deserialize)]
pub struct AppConfig {
    pub rpc: RpcConfig,
    pub kafka: KafkaConfig,
    pub postgres: PostgresConfig,
    pub metrics: MetricsConfig,
    pub factories: PathBuf,
    #[serde(default)]
    pub enrichment: EnrichmentConfig,
}

#[derive(Debug, Clone, Deserialize)]
pub struct RpcConfig {
    pub ws_url: String,
    #[serde(default)]
    pub http_url: Option<String>,
}

#[derive(Debug, Clone, Deserialize)]
pub struct KafkaConfig {
    pub brokers: String,
    pub transactional_id: String,
}

#[derive(Debug, Clone, Deserialize)]
pub struct PostgresConfig {
    pub url: String,
    pub max_connections: u32,
}

#[derive(Debug, Clone, Deserialize)]
pub struct MetricsConfig {
    pub bind: String,
}

#[derive(Debug, Clone, Default, Deserialize)]
pub struct EnrichmentConfig {
    /// Disable RPC enrichment entirely (use a stub source). Useful in dev.
    #[serde(default)]
    pub disable: bool,
    /// Path to bytecode signature JSON (4-byte selectors hex-encoded).
    #[serde(default)]
    pub bytecode_signatures: Option<PathBuf>,
}

pub fn load(path: &Path) -> Result<AppConfig> {
    let body = std::fs::read_to_string(path).with_context(|| format!("read {}", path.display()))?;
    let cfg: AppConfig =
        toml::from_str(&body).with_context(|| format!("parse {}", path.display()))?;
    Ok(cfg)
}
