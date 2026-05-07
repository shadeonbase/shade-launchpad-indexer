use anyhow::{Context, Result};
use clap::Args as ClapArgs;
use shade_indexer_bytecode::SignatureSet;
use shade_indexer_core::FactoryRegistry;
use shade_indexer_enrich::{EnrichmentSource, RpcEnrichmentSource, RpcSourceConfig};
use std::net::SocketAddr;
use std::path::PathBuf;
use std::sync::Arc;
use tracing::{info, warn};

use crate::config;
use crate::health;
use crate::pipeline::{self, StubSource};

#[derive(Debug, ClapArgs)]
pub struct Args {
    /// HTTP bind for /healthz + /readyz. Defaults to <metrics.bind ports + 1>.
    #[arg(long)]
    pub health_bind: Option<String>,
}

pub async fn run(config_path: PathBuf, args: Args) -> Result<()> {
    let cfg = config::load(&config_path)?;
    info!(path = %config_path.display(), "config loaded");

    install_metrics_exporter(&cfg.metrics.bind)?;

    let registry = FactoryRegistry::from_toml_path(&cfg.factories)
        .with_context(|| format!("factories: {}", cfg.factories.display()))?;
    info!(factories = registry.len(), "factory registry ready");

    let pipeline = if cfg.enrichment.disable {
        info!("enrichment disabled; using stub source");
        pipeline::launch(&cfg, registry, Arc::new(StubSource)).await?
    } else if let Some(http_url) = cfg.rpc.http_url.clone() {
        let signatures = match cfg.enrichment.bytecode_signatures.as_ref() {
            Some(p) => Arc::new(
                SignatureSet::from_json_file(p)
                    .with_context(|| format!("loading bytecode signatures from {}", p.display()))?,
            ),
            None => Arc::new(SignatureSet::builtin()),
        };
        let rpc_cfg = RpcSourceConfig {
            http_url,
            ..Default::default()
        };
        let source: Arc<dyn EnrichmentSource> =
            Arc::new(RpcEnrichmentSource::new(rpc_cfg, signatures).context("rpc source init")?);
        info!("RPC enrichment source online");
        pipeline::launch(&cfg, registry, source).await?
    } else {
        warn!("rpc.http_url not configured; falling back to stub enrichment");
        pipeline::launch(&cfg, registry, Arc::new(StubSource)).await?
    };

    // Health server.
    let health_addr: SocketAddr = args
        .health_bind
        .clone()
        .or_else(|| Some(default_health_bind(&cfg.metrics.bind)))
        .unwrap()
        .parse()
        .context("health_bind")?;
    let liveness = pipeline.liveness.clone();
    let health_h = tokio::spawn(async move {
        if let Err(e) = health::serve(health_addr, liveness).await {
            tracing::error!(error = ?e, "health server exited");
        }
    });

    info!("shade-indexer up; awaiting shutdown signal");
    pipeline::shutdown_signal().await;

    pipeline.subscriber_h.abort();
    health_h.abort();
    let _ = pipeline.fanout_h.await;
    let _ = pipeline.producer_h.await;
    let _ = pipeline.worker_h.await;
    info!("clean shutdown");
    Ok(())
}

fn install_metrics_exporter(bind: &str) -> Result<()> {
    use metrics_exporter_prometheus::PrometheusBuilder;
    let addr: SocketAddr = bind.parse().context("metrics.bind")?;
    PrometheusBuilder::new()
        .with_http_listener(addr)
        .install()
        .context("metrics exporter")?;
    info!(%addr, "prometheus exporter on /metrics");
    Ok(())
}

fn default_health_bind(metrics_bind: &str) -> String {
    let parsed: Option<SocketAddr> = metrics_bind.parse().ok();
    if let Some(addr) = parsed {
        let port = addr.port().wrapping_add(1);
        format!("{}:{}", addr.ip(), port)
    } else {
        "0.0.0.0:9091".into()
    }
}
