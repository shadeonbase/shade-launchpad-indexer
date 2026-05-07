use anyhow::{Context, Result};
use shade_indexer_core::{FactoryRegistry, LaunchpadSubscriber, NormalizedDeploy};
use shade_indexer_enrich::{EnrichStore, EnrichWorker, EnrichmentSource, HolderSnapshot};
use shade_indexer_kafka::{LaunchpadProducer, ProducerConfig};
use std::sync::Arc;
use tokio::signal;
use tokio::sync::mpsc;
use tokio::task::JoinHandle;
use tracing::{error, info, warn};

use crate::config::AppConfig;
use crate::health::Liveness;

pub struct Pipeline {
    pub liveness: Arc<Liveness>,
    pub subscriber_h: JoinHandle<()>,
    pub fanout_h: JoinHandle<()>,
    pub producer_h: JoinHandle<()>,
    pub worker_h: JoinHandle<()>,
}

pub async fn launch(
    cfg: &AppConfig,
    registry: FactoryRegistry,
    enrichment_source: Arc<dyn EnrichmentSource>,
) -> Result<Pipeline> {
    let liveness = Liveness::new();

    let (subscriber, mut deploy_rx) = LaunchpadSubscriber::new(&cfg.rpc.ws_url, registry);
    let (kafka_tx, kafka_rx) = mpsc::channel::<NormalizedDeploy>(4096);
    let (enrich_tx, enrich_rx) = mpsc::channel::<NormalizedDeploy>(4096);

    let producer = LaunchpadProducer::new(ProducerConfig {
        brokers: cfg.kafka.brokers.clone(),
        transactional_id: cfg.kafka.transactional_id.clone(),
        ..Default::default()
    })
    .context("kafka producer init")?;
    liveness.set_kafka(true);

    let store = EnrichStore::connect(&cfg.postgres.url, cfg.postgres.max_connections)
        .await
        .context("postgres connect")?;
    liveness.set_postgres(true);

    let worker = EnrichWorker::new(store, enrichment_source);

    let subscriber_h = tokio::spawn(async move {
        if let Err(e) = subscriber.run().await {
            error!(error = ?e, "subscriber exited with error");
        }
    });

    let liveness_for_fanout = liveness.clone();
    let fanout_h = tokio::spawn(async move {
        while let Some(d) = deploy_rx.recv().await {
            liveness_for_fanout.mark_event();
            if kafka_tx.send(d.clone()).await.is_err() {
                warn!("kafka sink closed");
                break;
            }
            if enrich_tx.send(d).await.is_err() {
                warn!("enrich sink closed");
                break;
            }
        }
    });

    let liveness_for_kafka = liveness.clone();
    let producer_h = tokio::spawn(producer_loop(producer, kafka_rx, liveness_for_kafka));
    let worker_h = tokio::spawn(worker.run(enrich_rx));

    Ok(Pipeline {
        liveness,
        subscriber_h,
        fanout_h,
        producer_h,
        worker_h,
    })
}

async fn producer_loop(
    producer: LaunchpadProducer,
    mut rx: mpsc::Receiver<NormalizedDeploy>,
    liveness: Arc<Liveness>,
) {
    while let Some(deploy) = rx.recv().await {
        match producer.publish(&deploy).await {
            Ok(()) => liveness.set_kafka(true),
            Err(e) => {
                liveness.set_kafka(false);
                warn!(error = ?e, "kafka publish failed; dropping message");
                metrics::counter!("shade_indexer_kafka_drop").increment(1);
            }
        }
    }
}

/// Stub enrichment source — returns an empty snapshot. Used when
/// `enrichment.disable = true` or for offline tests.
pub struct StubSource;

#[async_trait::async_trait]
impl EnrichmentSource for StubSource {
    async fn fetch(&self, _deploy: &NormalizedDeploy) -> anyhow::Result<HolderSnapshot> {
        Ok(HolderSnapshot {
            balances: vec![],
            liquidity_usd: 0.0,
            fdv_usd: 0.0,
            liquidity_locked: false,
            bytecode_flags: 0,
        })
    }
}

pub async fn shutdown_signal() {
    let ctrl_c = async { signal::ctrl_c().await.expect("ctrl_c install") };
    #[cfg(unix)]
    let term = async {
        signal::unix::signal(signal::unix::SignalKind::terminate())
            .expect("SIGTERM install")
            .recv()
            .await;
    };
    #[cfg(not(unix))]
    let term = std::future::pending::<()>();
    tokio::select! { _ = ctrl_c => {}, _ = term => {} }
    info!("shutdown signal received");
}
