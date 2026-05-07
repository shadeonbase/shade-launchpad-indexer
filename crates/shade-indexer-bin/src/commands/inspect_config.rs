use anyhow::{Context, Result};
use serde::Serialize;
use shade_indexer_core::FactoryRegistry;
use std::path::PathBuf;

use crate::config;

#[derive(Serialize)]
struct Inspection {
    config_path: String,
    rpc_ws_url: String,
    rpc_http_url: Option<String>,
    kafka_brokers: String,
    kafka_transactional_id: String,
    postgres_url_redacted: String,
    metrics_bind: String,
    factories: Vec<FactorySummary>,
}

#[derive(Serialize)]
struct FactorySummary {
    launchpad: String,
    address: String,
    event_topic: String,
}

pub async fn run(config_path: PathBuf) -> Result<()> {
    let cfg = config::load(&config_path)?;
    let registry = FactoryRegistry::from_toml_path(&cfg.factories)
        .with_context(|| format!("factories: {}", cfg.factories.display()))?;

    let inspection = Inspection {
        config_path: config_path.display().to_string(),
        rpc_ws_url: cfg.rpc.ws_url,
        rpc_http_url: cfg.rpc.http_url,
        kafka_brokers: cfg.kafka.brokers,
        kafka_transactional_id: cfg.kafka.transactional_id,
        postgres_url_redacted: redact_url(&cfg.postgres.url),
        metrics_bind: cfg.metrics.bind,
        factories: registry
            .specs()
            .map(|s| FactorySummary {
                launchpad: s.launchpad.to_string(),
                address: format!("{:#x}", s.address),
                event_topic: format!("{:#x}", s.event_topic),
            })
            .collect(),
    };

    println!("{}", serde_json::to_string_pretty(&inspection)?);
    Ok(())
}

/// Strip credentials from a postgres URL for safe display.
fn redact_url(url: &str) -> String {
    if let Some((scheme, rest)) = url.split_once("://") {
        if let Some(at) = rest.find('@') {
            return format!("{scheme}://***:***@{}", &rest[at + 1..]);
        }
    }
    url.into()
}
